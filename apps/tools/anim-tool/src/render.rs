use crate::{
    app::FIXED_FPS,
    atlas_generator::{build_atlas, save_atlas_png, AtlasGenConfig, FontAtlas, FontId},
    common_state::{Globals, RenderData, SDF_HEADER_SIZE},
    text_layout::{glyphs_to_quads, layout_text, LayoutOptions, WrapMode},
};
use al_math::{
    matrix::Mat2,
    sdf::{Sdf2dGroup, Sdf2dOp, Sdf2dPrim, SDF_GROUP_SIZE, SDF_OP_SIZE, SDF_PRIM_SIZE},
    transform::Transform2d,
    vec::{Vec2, Vec3, Vec4},
};
use al_skeleton::skeleton::ColliderRef;
use std::path::Path;
use wgpu::{Surface, SurfaceConfiguration};

pub const MAX_TEXT_QUADS: usize = 4096;
pub const MAX_SDF_PRIMS: usize = 1024;

// Mirrors the TextQuad struct in sdf_text.wgsl (32-byte aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: [f32; 4],
}

// Mirrors the TextGlobals struct in sdf_text.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextGlobals {
    pub atlas_size: [f32; 2],
    pub atlas_type: f32, // 0 = MSDF, 1 = MTSDF
    pub _pad: f32,
}

/// A single text rendering instance with its own text, font size, position, and color.
#[derive(Clone, Copy, Debug)]
pub struct TextInstance {
    pub text: &'static str,
    pub font_id: FontId,
    pub font_size: f32,
    pub position: [f32; 2],
    /// Maximum available width for wrapping from position.x (NDC units). None = no wrapping.
    pub max_width: Option<f32>,
    /// Wrapping mode (PerChar or PerWord). Defaults to PerChar.
    pub wrap_mode: WrapMode,
    pub color: [f32; 4],
}

pub struct BooleanTargets {
    pub color_view: wgpu::TextureView,
    pub union_view: wgpu::TextureView,
    pub subtract_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl BooleanTargets {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;

        let mk = |label: &'static str, format: wgpu::TextureFormat| {
            device
                .create_texture(&wgpu::wgt::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Self {
            color_view: mk("Boolean Color", color_format),
            union_view: mk("Boolean Union D", wgpu::TextureFormat::R16Float),
            subtract_view: mk("Boolean Subtract D", wgpu::TextureFormat::R16Float),
            width: w,
            height: h,
        }
    }
}

fn make_composite_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    targets: &BooleanTargets,
    globals_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Composite Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&targets.union_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&targets.subtract_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&targets.color_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: globals_buffer.as_entire_binding(),
            },
        ],
    })
}

fn depth_tint(color: Vec4, depth: f32) -> Vec4 {
    let f = (1. - depth * 0.22).clamp(0.35, 1.25);
    Vec4::new(color.x * f, color.y * f, color.z * f, color.w)
}

pub struct RenderState {
    pub surface: Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: SurfaceConfiguration,
    pub sdf_pipeline: wgpu::RenderPipeline,
    pub sdf_ops_pipeline: wgpu::RenderPipeline,
    pub sdf_groups_pipeline: wgpu::RenderPipeline,
    pub globals_buffer: wgpu::Buffer,
    pub union_buffer: wgpu::Buffer,
    pub union_bind_group: wgpu::BindGroup,
    pub subtract_buffer: wgpu::Buffer,
    pub subtract_bind_group: wgpu::BindGroup,
    pub overlay_buffer: wgpu::Buffer,
    pub overlay_bind_group: wgpu::BindGroup,
    pub ops_buffer: wgpu::Buffer,
    pub groups_buffer: wgpu::Buffer,
    pub bg_pipeline: wgpu::RenderPipeline,
    pub solid_bg_pipeline: wgpu::RenderPipeline,
    pub debug_bg_pipeline: wgpu::RenderPipeline,
    pub bg_bind_group: wgpu::BindGroup,
    pub vfx_pipeline: wgpu::RenderPipeline,
    pub vfx_buffer: wgpu::Buffer,
    pub vfx_bind_group: wgpu::BindGroup,
    // Boolean Operations
    pub boolean_targets: BooleanTargets,
    pub union_pipeline: wgpu::RenderPipeline,
    pub subtract_pipeline: wgpu::RenderPipeline,
    pub composite_pipeline: wgpu::RenderPipeline,
    pub composite_bgl: wgpu::BindGroupLayout,
    pub composite_bind_group: wgpu::BindGroup,
    // Glyphon
    pub font_system: glyphon::FontSystem,
    pub swash_cache: glyphon::SwashCache,
    pub _cache: glyphon::Cache,
    pub atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,
    pub viewport: glyphon::Viewport,
    // MTSDF Text
    pub text_pipeline: wgpu::RenderPipeline,
    pub text_quads_buffer: wgpu::Buffer,
    pub text_bind_group: wgpu::BindGroup,
    pub text_atlas: FontAtlas,
    pub text_atlas_type: TextAtlasType,
    pub text_globals_buffer: wgpu::Buffer,
    pub text_bind_group_layout: wgpu::BindGroupLayout,
}

//TODO: Remove
/// Type of text atlas in use.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextAtlasType {
    /// Pre-baked MSDF atlas loaded from PNG + JSON files.
    #[default]
    Msdf,
    /// MTSDF atlas generated at runtime.
    Mtsdf,
}

impl RenderState {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let capacilities = surface.get_capabilities(&adapter);
        let format = capacilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capacilities.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Buffer"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let designer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SDF Designer Shader"),
            source: wgpu::ShaderSource::Wgsl(designer_sdf_source().into()),
        });

        // MTSDF Text Shader
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MTSDF Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sdf_text.wgsl").into()),
        });

        // MTSDF Text Bind Group Layout
        let text_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, //REVIEW: needed for Vertex only?
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Text quads buffer (per-instance quads).
        let text_quads_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Quads Buffer"),
            size: (MAX_TEXT_QUADS * std::mem::size_of::<TextQuad>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Text globals buffer (atlas size uniform).
        let text_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Globals Buffer"),
            size: std::mem::size_of::<TextGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Generate MTSDF atlas from `.ttf` file
        let atlas_gen_config = AtlasGenConfig::default();
        let chars = (' '..='~').collect::<Vec<_>>();
        let fonts = [
            (
                FontId::Normal,
                Path::new("apps/tools/anim-tool/src/font_files/asul/Asul-Regular.ttf"),
            ),
            (
                FontId::Bold,
                Path::new("apps/tools/anim-tool/src/font_files/asul/Asul-Bold.ttf"),
            ),
        ];
        let text_atlas = build_atlas(&device, &queue, &fonts, &chars, &atlas_gen_config);
        let text_atlas_name = "test_atlas.png";
        save_atlas_png(
            &device,
            &queue,
            &text_atlas.texture,
            atlas_gen_config.atlas_size,
            atlas_gen_config.atlas_size,
            Path::new(text_atlas_name),
        )
        .expect(&format!("Failed to save atlas PNG as {text_atlas_name}"));
        eprintln!("Saved test atlas PNG as {text_atlas_name}");

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&text_bgl)],
            immediate_size: 0,
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MSDF Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_text"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_text"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let bg_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BG Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bg_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BG Bind Group"),
            layout: &bg_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BG Pipeline Layout"),
            bind_group_layouts: &[Some(&bg_bgl)],
            immediate_size: 0,
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BG Pipeline"),
            layout: Some(&bg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let solid_bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid BG Pipeline"),
            layout: Some(&bg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_bg_solid"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let sdf_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let sdf_buffer_size = (SDF_HEADER_SIZE + MAX_SDF_PRIMS * SDF_PRIM_SIZE) as u64;
        let mk_buf = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: sdf_buffer_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let union_buffer = mk_buf("SDF Union Buffer");
        let subtract_buffer = mk_buf("SDF Subtract Buffer");
        let vfx_buffer = mk_buf("SDF Vfx Buffer");
        let overlay_buffer = mk_buf("SDF Overlay Buffer");

        let ops_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Ops Buffer"),
            size: (MAX_SDF_PRIMS * SDF_OP_SIZE) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let groups_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Groups Buffer"),
            size: (MAX_SDF_PRIMS * SDF_GROUP_SIZE) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mk_bgl = |label: &str, buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &sdf_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ops_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: groups_buffer.as_entire_binding(),
                    },
                ],
            })
        };

        let union_bind_group = mk_bgl("SDF Union Bind Group", &union_buffer);
        let subtract_bind_group = mk_bgl("SDF Subtract Bind Group", &subtract_buffer);
        let overlay_bind_group = mk_bgl("SDF Overlay Bind Group", &overlay_buffer);

        let sdf_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[Some(&sdf_bgl)],
            immediate_size: 0,
        });

        let sdf_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let sdf_ops_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Ops Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_main_ops"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_main_ops"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let sdf_groups_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Groups Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_groups"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_groups"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let debug_bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug BG Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_debug_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let vfx_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Vfx Bind Group"),
            layout: &sdf_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vfx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ops_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: groups_buffer.as_entire_binding(),
                },
            ],
        });

        // MTSDF Text Bind Group
        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MSDF Text Bind Group"),
            layout: &text_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: text_quads_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&text_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&text_atlas.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: text_globals_buffer.as_entire_binding(),
                },
            ],
        });

        let vfx_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF Vfx Pipeline Layout"),
            bind_group_layouts: &[Some(&sdf_bgl)],
            immediate_size: 0,
        });

        let vfx_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Vfx Pipeline"),
            layout: Some(&vfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[], // vertices synthesized from vertex_index
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let union_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Boolean Union Pipeline"),
            layout: Some(&sdf_pipeline_layout), // reuses globals + prims
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_boolean_union"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R16Float,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Min,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Min,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let subtract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Boolean Subtract Pipeline"),
            layout: Some(&sdf_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_boolean_subtract"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R16Float,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Composite BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Composite Layout"),
            bind_group_layouts: &[Some(&composite_bgl)],
            immediate_size: 0,
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Composite Pipeline"),
            layout: Some(&composite_layout),
            vertex: wgpu::VertexState {
                module: &designer_shader,
                entry_point: Some("vs_composite"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &designer_shader,
                entry_point: Some("fs_composite"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        let boolean_targets = BooleanTargets::new(
            &device,
            window.inner_size().width,
            window.inner_size().height,
            config.format,
        );
        let composite_bind_group =
            make_composite_bg(&device, &composite_bgl, &boolean_targets, &globals_buffer);

        // Glyphon
        let font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas = glyphon::TextAtlas::new(&device, &queue, &cache, config.format);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            surface,
            device,
            queue,
            config,
            sdf_pipeline,
            sdf_ops_pipeline,
            sdf_groups_pipeline,
            globals_buffer,
            union_buffer,
            union_bind_group,
            subtract_buffer,
            subtract_bind_group,
            overlay_buffer,
            overlay_bind_group,
            ops_buffer,
            groups_buffer,
            bg_pipeline,
            solid_bg_pipeline,
            debug_bg_pipeline,
            bg_bind_group,
            vfx_pipeline,
            vfx_buffer,
            vfx_bind_group,
            boolean_targets,
            union_pipeline,
            subtract_pipeline,
            composite_pipeline,
            composite_bgl,
            composite_bind_group,
            font_system,
            swash_cache,
            _cache: cache,
            atlas,
            text_renderer,
            viewport,
            text_pipeline,
            text_quads_buffer,
            text_bind_group,
            text_atlas: text_atlas,
            text_atlas_type: TextAtlasType::Mtsdf,
            text_globals_buffer,
            text_bind_group_layout: text_bgl,
        }
        //atlas.texture, atlas.view, atlas.sampler, atlas.glyphs
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);

        self.boolean_targets = BooleanTargets::new(&self.device, width, height, self.config.format);
        self.composite_bind_group = make_composite_bg(
            &self.device,
            &self.composite_bgl,
            &self.boolean_targets,
            &self.globals_buffer,
        );
    }

    pub fn render_designer(&mut self, d: &mut crate::designer_state::DesignerState) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view = surface_texture.texture.create_view(&Default::default());

        let resolution = [self.config.width as f32, self.config.height as f32];
        d.resolution = resolution;

        // Cursor -> world (matches GameState::update).
        let cx = d.cursor_px[0] - 0.5 * resolution[0];
        let cy = d.cursor_px[1] - 0.5 * resolution[1];
        d.cursor_world = Vec2::new(
            d.camera.pos[0] + (cx / resolution[1]) * d.camera.view_size,
            d.camera.pos[1] + (-cy / resolution[1]) * d.camera.view_size,
        );

        // Globals
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::bytes_of(&Globals::new(
                resolution,
                d.camera.pos,
                d.cursor_world,
                d.camera.view_size,
            )),
        );

        // ---- Build the rig primitive list ----
        let mut union_prims = Vec::with_capacity(MAX_SDF_PRIMS);
        #[allow(unused)]
        let mut subtract_prims = Vec::with_capacity(MAX_SDF_PRIMS);
        let mut vfx_prims = Vec::with_capacity(MAX_SDF_PRIMS);
        let mut overlay_prims = Vec::with_capacity(MAX_SDF_PRIMS);

        // Onion skins (always lowest).
        if d.onion_keys > 0 {
            let clip = d.clip();
            let keys = clip.keyframes();
            if !keys.is_empty() {
                let curr = keys.partition_point(|kf| kf.time <= d.time);

                match d.onion_mode {
                    crate::designer_state::OnionMode::Keys => {
                        for offset in 1..=d.onion_keys {
                            let alpha = 0.22 / offset as f32;
                            if let Some(idx) = curr.checked_sub(offset) {
                                push_body_prims(
                                    d,
                                    &mut overlay_prims,
                                    &d.skeleton.world_transforms(&keys[idx].pose),
                                    Vec4::new(1., 0.35, 0.35, 1.),
                                    alpha,
                                );
                            }
                            let next_idx = curr + offset - 1;
                            if next_idx < keys.len() {
                                push_body_prims(
                                    d,
                                    &mut overlay_prims,
                                    &d.skeleton.world_transforms(&keys[next_idx].pose),
                                    Vec4::new(0.35, 0.55, 1., 1.),
                                    alpha,
                                );
                            }
                        }
                    }
                    crate::designer_state::OnionMode::Time => {
                        for k in 1..=d.onion_keys {
                            let dt = k as f32 / FIXED_FPS;
                            let alpha = 0.22 / k as f32;
                            for (t, color) in [
                                (d.time - dt, Vec4::new(1., 0.35, 0.35, 1.)),
                                (d.time + dt, Vec4::new(0.35, 0.55, 1., 1.)),
                            ] {
                                if t < 0. || t > clip.duration() {
                                    continue;
                                }
                                push_body_prims(
                                    d,
                                    &mut overlay_prims,
                                    &d.skeleton.world_transforms(&clip.sample(t)),
                                    color,
                                    alpha,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Live body (bottom layer).
        let world = d.skeleton.world_transforms(&d.pose);
        push_body_prims(
            d,
            &mut union_prims,
            &world,
            Vec4::new(0.82, 0.85, 0.92, 1.),
            1.,
        );

        // Extra colliders above body
        if d.show_extras {
            for (i, extra) in d.skeleton.extras().iter().enumerate() {
                let shape = extra.shape();
                let (color, stroke_color) = if d.extra_active.get(i).copied().unwrap_or(false) {
                    let col = Vec3::new(0.3, 0.95, 0.3);
                    (col.extend(1.), (col * 0.4).extend(1.))
                } else {
                    let col = Vec3::new(0.3, 0.55, 0.3);
                    (col.extend(0.35), (col * 0.4).extend(0.35))
                };
                overlay_prims.push(if let Some(idx) = extra.parent() {
                    shape.to_prim(world[idx], color, stroke_color)
                } else {
                    shape.to_prim(Transform2d::IDENTITY, color, stroke_color)
                });
            }
        }

        // Vfx above colliders
        if d.show_vfx {
            vfx_prims.extend_from_slice(&d.clip().vfx_instances(d.time, &d.skeleton));
        }

        // Joint markers on top — small dots, selected joint highlighted.
        for (i, _) in d.skeleton.joints().iter().enumerate() {
            let c = world[i].transform_point(Vec2::ZERO);
            let selected = d.selected == Some(ColliderRef::Joint(i));
            overlay_prims.push(Sdf2dPrim::circle(
                c,
                if selected { 0.016 } else { 0.008 },
                Mat2::IDENTITY.to_cols_array(),
                if selected {
                    Vec4::new(1., 0.75, 0.15, 1.)
                } else {
                    Vec4::new(1., 1., 1., 0.85)
                },
                if selected {
                    Vec4::new(0.4, 0.3, 0.06, 1.)
                } else {
                    Vec4::new(0.4, 0.4, 0.4, 0.85)
                },
            ));
        }

        // Cursor marker so you can see what you're pointing at.
        overlay_prims.push(Sdf2dPrim::circle(
            d.cursor_world,
            0.006,
            Mat2::IDENTITY.to_cols_array(),
            Vec4::new(0.4, 1., 0.6, 1.),
            Vec4::new(0.16, 0.4, 0.24, 1.),
        ));

        let mut encoder = self.device.create_command_encoder(&Default::default());
        // ----- Background Pass -----
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BG Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.bg_pipeline);
            pass.set_bind_group(0, &self.bg_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ----- Union Pass -----
        debug_assert!(
            union_prims.len() <= MAX_SDF_PRIMS,
            "union prim overflow: {} > {}",
            union_prims.len(),
            MAX_SDF_PRIMS
        );
        let n_union = union_prims.len().min(MAX_SDF_PRIMS);
        {
            write_sdf(&self.queue, &self.union_buffer, &union_prims);

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Boolean Union Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.boolean_targets.color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.boolean_targets.union_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 65504.,
                                g: 0.,
                                b: 0.,
                                a: 0.,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.union_pipeline);
            pass.set_bind_group(0, &self.union_bind_group, &[]);
            pass.draw(0..6, 0..n_union as u32);
        }

        // ----- Subtract Pass -----
        debug_assert!(
            subtract_prims.len() <= MAX_SDF_PRIMS,
            "subtract prim overflow: {} > {}",
            subtract_prims.len(),
            MAX_SDF_PRIMS
        );
        let n_subtract = subtract_prims.len().min(MAX_SDF_PRIMS);
        {
            write_sdf(&self.queue, &self.subtract_buffer, &subtract_prims);

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Boolean Subtract Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.boolean_targets.subtract_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: -65504.,
                            g: 0.,
                            b: 0.,
                            a: 0.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.subtract_pipeline);
            pass.set_bind_group(0, &self.subtract_bind_group, &[]);
            pass.draw(0..6, 0..n_subtract as u32);
        }

        // ----- Composite Pass -----
        if n_union > 0 || n_subtract > 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Boolean Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ----- Vfx Pass -----
        {
            debug_assert!(
                vfx_prims.len() <= MAX_SDF_PRIMS,
                "vfx prim overflow: {} > {}",
                vfx_prims.len(),
                MAX_SDF_PRIMS
            );
            let n_vfx = vfx_prims.len().min(MAX_SDF_PRIMS);
            if n_vfx > 0 {
                write_sdf(&self.queue, &self.vfx_buffer, &vfx_prims);

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("VFX Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.vfx_pipeline);
                pass.set_bind_group(0, &self.vfx_bind_group, &[]);
                pass.draw(0..6, 0..n_vfx as u32);
            }
        }

        // ----- Overlay Pass -----
        {
            debug_assert!(
                overlay_prims.len() <= MAX_SDF_PRIMS,
                "overlay prim overflow: {} > {}",
                overlay_prims.len(),
                MAX_SDF_PRIMS
            );
            let n_overlay = overlay_prims.len().min(MAX_SDF_PRIMS);
            if n_overlay > 0 {
                write_sdf(&self.queue, &self.overlay_buffer, &overlay_prims);

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Overlay Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.sdf_pipeline);
                pass.set_bind_group(0, &self.overlay_bind_group, &[]);
                pass.draw(0..6, 0..n_overlay as u32);
            }
        }

        // ----- Text Pass -----
        {
            let mut text_buffer =
                glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(14., 18.));
            text_buffer.set_size(
                Some(self.config.width as f32),
                Some(self.config.height as f32),
            );

            text_buffer.set_text(
                &d.hud_string(),
                &glyphon::Attrs::new().family(glyphon::Family::Monospace),
                glyphon::Shaping::Advanced,
                None,
            );
            text_buffer.shape_until_scroll(&mut self.font_system, false);

            self.viewport.update(
                &self.queue,
                glyphon::Resolution {
                    width: self.config.width,
                    height: self.config.height,
                },
            );

            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    vec![glyphon::TextArea {
                        buffer: &mut text_buffer,
                        left: 12.,
                        top: 12.,
                        scale: 1.,
                        bounds: glyphon::TextBounds {
                            left: 0,
                            top: 0,
                            right: 800,
                            bottom: 200,
                        },
                        default_color: glyphon::Color::rgb(230, 230, 240),
                        custom_glyphs: &[],
                    }],
                    &mut self.swash_cache,
                )
                .unwrap();

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Designer Text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .unwrap();
        }

        // ----- MSDF Text Pass -----
        {
            // Write text quads to storage buffer for MVP demo.
            // These quads render "Hello MSDF" as three demo glyph quads.

            let quads: [TextQuad; 3] = [
                TextQuad {
                    min: [50., 50.].into(),
                    max: [150., 100.].into(),
                    uv_min: [0.0, 0.0].into(),
                    uv_max: [0.33, 1.0].into(),
                    color: [1., 1., 1., 1.],
                },
                TextQuad {
                    min: [160., 50.].into(),
                    max: [260., 100.].into(),
                    uv_min: [0.33, 0.0].into(),
                    uv_max: [0.66, 1.0].into(),
                    color: [1., 0.5, 0.5, 1.],
                },
                TextQuad {
                    min: [270., 50.].into(),
                    max: [370., 100.].into(),
                    uv_min: [0.66, 0.0].into(),
                    uv_max: [1.0, 1.0].into(),
                    color: [0.5, 1., 0.5, 1.],
                },
            ];

            self.queue
                .write_buffer(&self.text_quads_buffer, 0, bytemuck::cast_slice(&quads));

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MSDF Text Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.text_pipeline);
            pass.set_bind_group(0, &self.text_bind_group, &[]);
            pass.draw(0..6, 0..quads.len() as u32);
        }

        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        self.queue.present(surface_texture);
    }

    pub fn render_showcase(&mut self, to_render: RenderData) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view = surface_texture.texture.create_view(&Default::default());

        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::bytes_of(&to_render.globals),
        );

        write_sdf(&self.queue, &self.overlay_buffer, &to_render.sdfs);
        write_ops(&self.queue, &self.ops_buffer, &to_render.ops);
        write_groups(&self.queue, &self.groups_buffer, &to_render.groups);

        let mut encoder = self.device.create_command_encoder(&Default::default());
        // ----- Background Pass -----
        {
            let aux = to_render.globals.aux;
            let (pipeline, bind_group) = if aux >= 10. {
                (&self.debug_bg_pipeline, &self.overlay_bind_group)
            } else if aux > 0.5 {
                (&self.solid_bg_pipeline, &self.bg_bind_group)
            } else {
                (&self.bg_pipeline, &self.bg_bind_group)
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("BG Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // ----- Showcase Pass -----
        {
            let sdf_len = to_render.sdfs.len();
            debug_assert!(
                sdf_len <= MAX_SDF_PRIMS,
                "overlay prim overflow: {} > {}",
                sdf_len,
                MAX_SDF_PRIMS
            );
            let n_showcase = sdf_len.min(MAX_SDF_PRIMS);
            if n_showcase > 0 {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Showcase Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.sdf_ops_pipeline);
                pass.set_bind_group(0, &self.overlay_bind_group, &[]);
                pass.draw(0..6, 0..n_showcase as u32);
            }

            let n_groups = to_render.groups.len();
            debug_assert!(
                n_groups <= MAX_SDF_PRIMS,
                "groups overflow: {} > {}",
                n_groups,
                MAX_SDF_PRIMS
            );
            let n_groups = n_groups.min(MAX_SDF_PRIMS);
            if n_groups > 0 {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Groups Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.sdf_groups_pipeline);
                pass.set_bind_group(0, &self.overlay_bind_group, &[]);
                pass.draw(0..6, 0..n_groups as u32);
            }
        }

        // ----- Text Pass -----
        {
            let mut text_buffer =
                glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(14., 18.));
            text_buffer.set_size(
                Some(self.config.width as f32),
                Some(self.config.height as f32),
            );

            text_buffer.set_text(
                &to_render.hud_string,
                &glyphon::Attrs::new().family(glyphon::Family::Monospace),
                glyphon::Shaping::Advanced,
                None,
            );
            text_buffer.shape_until_scroll(&mut self.font_system, false);

            self.viewport.update(
                &self.queue,
                glyphon::Resolution {
                    width: self.config.width,
                    height: self.config.height,
                },
            );

            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    vec![glyphon::TextArea {
                        buffer: &mut text_buffer,
                        left: 12.,
                        top: 12.,
                        scale: 1.,
                        bounds: glyphon::TextBounds {
                            left: 0,
                            top: 0,
                            right: 800,
                            bottom: 200,
                        },
                        default_color: glyphon::Color::rgb(230, 230, 240),
                        custom_glyphs: &[],
                    }],
                    &mut self.swash_cache,
                )
                .unwrap();

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Designer Text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .unwrap();
        }

        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        self.queue.present(surface_texture);
    }
}

fn strip_editor_meta(source: &str) -> String {
    const START_MARKER: &str = "// ----- Meta Start -----";
    const END_MARKER: &str = "// ----- Meta End -----";

    // Find the start of the editor meta block.
    let start_pos = match source.find(START_MARKER) {
        Some(pos) => pos,
        None => {
            // No Start marker: fall back to stripping everything above Meta End.
            match source.find(END_MARKER) {
                Some(pos) => {
                    let rest = &source[pos + END_MARKER.len()..];
                    return rest
                        .strip_prefix("\r\n")
                        .or_else(|| rest.strip_prefix('\n'))
                        .unwrap_or(rest)
                        .to_string();
                }
                None => return source.to_string(),
            }
        }
    };

    // Find the end of the editor meta block.
    let end_search = &source[start_pos..];
    let end_pos = match end_search.find(END_MARKER) {
        Some(pos) => start_pos + pos + END_MARKER.len(),
        None => source.len(),
    };

    // Keep everything before Meta Start, skip to after Meta End.
    let before = &source[..start_pos];
    let after = &source[end_pos..];
    let after = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after);

    // Join before and after with a newline between them.
    if before.is_empty() {
        after.to_string()
    } else if after.is_empty() {
        before.to_string()
    } else {
        format!("{before}\n{after}")
    }
}

pub fn designer_sdf_source() -> String {
    let functions = include_str!("sdf_functions.wgsl");
    let body = strip_editor_meta(include_str!("designer_sdf.wgsl"));
    format!("{functions}\n{body}")
}

fn write_sdf(queue: &wgpu::Queue, buffer: &wgpu::Buffer, prims: &[Sdf2dPrim]) {
    debug_assert!(
        prims.len() <= MAX_SDF_PRIMS,
        "prim overflow: {} > {}",
        prims.len(),
        MAX_SDF_PRIMS,
    );
    let n = prims.len().min(MAX_SDF_PRIMS);
    queue.write_buffer(
        buffer,
        0,
        bytemuck::bytes_of(&crate::common_state::SdfHeader::new(n as u32)),
    );
    if n > 0 {
        queue.write_buffer(
            buffer,
            SDF_HEADER_SIZE as u64,
            bytemuck::cast_slice(&prims[..n]),
        );
    }
}

fn write_ops(queue: &wgpu::Queue, buffer: &wgpu::Buffer, ops: &[Sdf2dOp]) {
    debug_assert!(
        ops.len() <= MAX_SDF_PRIMS,
        "ops overflow: {} > {}",
        ops.len(),
        MAX_SDF_PRIMS,
    );
    let n = ops.len().min(MAX_SDF_PRIMS);
    if n > 0 {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&ops[..n]));
    }
}

fn write_groups(queue: &wgpu::Queue, buffer: &wgpu::Buffer, groups: &[Sdf2dGroup]) {
    debug_assert!(
        groups.len() <= MAX_SDF_PRIMS,
        "groups overflow: {} > {}",
        groups.len(),
        MAX_SDF_PRIMS
    );
    let n = groups.len().min(MAX_SDF_PRIMS);
    if n > 0 {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&groups[..n]));
    }
}

fn push_body_prims(
    d: &crate::designer_state::DesignerState,
    prims: &mut Vec<Sdf2dPrim>,
    world: &[Transform2d],
    base_color: Vec4,
    alpha_mul: f32,
) {
    let joints = d.skeleton.joints();
    let bones = d.skeleton.bones();
    let mut batch = Vec::with_capacity(joints.len() + bones.len());
    for (i, joint) in joints.iter().enumerate() {
        let rad = joint.radius();
        if rad <= 0. {
            continue;
        }
        let depth = joint.depth();
        let col = depth_tint(base_color, depth);
        batch.push((
            depth,
            Sdf2dPrim::circle(
                world[i].transform_point(Vec2::ZERO),
                rad,
                world[i]
                    .linear
                    .try_inverse()
                    .unwrap_or(Mat2::IDENTITY)
                    .to_cols_array(),
                Vec4::new(col.x, col.y, col.z, col.w * alpha_mul),
                Vec4::new(col.x * 0.4, col.y * 0.4, col.z * 0.4, col.w * alpha_mul),
            ),
        ));
    }

    for bone in bones {
        let depth = bone.depth();
        let col = depth_tint(base_color, depth);
        let w = world[bone.origin_index()];
        let wo = w.transform_point(Vec2::ZERO);
        let wt = world[bone.tip_index()].transform_point(Vec2::ZERO);
        let dir = wt - wo;
        let length = dir.length();
        let angle = dir.y.atan2(dir.x);
        let world_radius =
            al_skeleton::skeleton::world_bone_radius(w.linear, bone.radius(), wo, wt);
        let m_inv = Mat2::from_angle(-angle).to_cols_array();
        batch.push((
            depth,
            Sdf2dPrim::capsule(
                wo,
                length,
                world_radius, //bone.radius(),
                m_inv,
                Vec4::new(col.x, col.y, col.z, col.w * alpha_mul),
                Vec4::new(col.x, col.y, col.z, col.w * alpha_mul),
            ),
        ));
    }

    batch.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    prims.extend(batch.into_iter().map(|(_, p)| p));
}

impl RenderState {
    pub fn render_text(&mut self, text_instances: &[TextInstance], cursor_px: [f32; 2], zoom: f32) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view = surface_texture.texture.create_view(&Default::default());

        // Set up globals for text rendering (cursor world computed here).
        let res = [self.config.width as f32, self.config.height as f32];
        let px_per_unit = res[1] / 2.0;
        let cx = cursor_px[0] - 0.5 * res[0];
        let cy = cursor_px[1] - 0.5 * res[1];
        let cursor_world = Vec2::new(cx / px_per_unit, -cy / px_per_unit);

        let globals = Globals {
            resolution: Vec2::new(res[0], res[1]),
            camera_pos: Vec2::ZERO,
            cursor_world,
            view_size: 2.0,
            aux: 0.0,
        };

        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Write atlas size to text globals uniform.
        let text_globals = TextGlobals {
            atlas_size: [self.text_atlas.size as f32, self.text_atlas.size as f32],
            atlas_type: self.text_atlas_type as u32 as f32,
            _pad: 0.0,
        };
        self.queue.write_buffer(
            &self.text_globals_buffer,
            0,
            bytemuck::bytes_of(&text_globals),
        );

        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Clear background.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Background"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.solid_bg_pipeline);
            pass.set_bind_group(0, &self.bg_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Set up globals for text rendering (cursor world computed here).
        let res = [self.config.width as f32, self.config.height as f32];
        let px_per_unit = res[1] / 2.0;
        let cx = cursor_px[0] - 0.5 * res[0];
        let cy = cursor_px[1] - 0.5 * res[1];
        let cursor_world = Vec2::new(cx / px_per_unit, -cy / px_per_unit);
        let cursor_radius_world = 0.008;

        let globals = Globals {
            resolution: Vec2::new(res[0], res[1]),
            camera_pos: Vec2::ZERO,
            cursor_world,
            view_size: 2.0,
            aux: 0.0,
        };

        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Write atlas size to text globals uniform.
        let text_globals = TextGlobals {
            atlas_size: [self.text_atlas.size as f32, self.text_atlas.size as f32],
            atlas_type: self.text_atlas_type as u32 as f32,
            _pad: 0.0,
        };
        self.queue.write_buffer(
            &self.text_globals_buffer,
            0,
            bytemuck::bytes_of(&text_globals),
        );

        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Clear background.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Background"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.solid_bg_pipeline);
            pass.set_bind_group(0, &self.bg_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // MTSDF Text Pass - shape text using atlas data and render dynamic quads.
        // Maps Unicode chars directly to glyph entries in the atlas.
        // Builds all quads from all text instances.
        let mut all_quads: Vec<TextQuad> = Vec::new();
        for instance in text_instances.iter() {
            // Apply zoom to font size.
            let zoomed_font_size = instance.font_size * zoom;

            // Shape text into glyphs using atlas data.
            let shaped = crate::text_layout::shape_text(
                &self.text_atlas.glyphs,
                instance.font_id,
                instance.text,
                self.text_atlas.em_size,
            );

            // Layout shaped glyphs into lines.
            // Start at the passed position in NDC space.
            let options = LayoutOptions::new(zoomed_font_size)
                .with_max_width(instance.max_width.unwrap_or(1.8)) // Leave margins on left/right
                .with_line_height(1.2)
                .with_wrap_mode(instance.wrap_mode);
            let lines = layout_text(
                &shaped,
                &options,
                instance.position[0],
                instance.position[1],
            );

            // Convert layout into TextQuads in NDC space.
            let quads = glyphs_to_quads(
                &lines,
                &self.text_atlas.glyphs,
                instance.font_id,
                self.text_atlas.em_size,
                zoomed_font_size,
                instance.color,
            );
            all_quads.extend(quads);
        }

        if !all_quads.is_empty() {
            self.queue
                .write_buffer(&self.text_quads_buffer, 0, bytemuck::cast_slice(&all_quads));
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("MSDF Text Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.text_bind_group, &[]);
                pass.draw(0..6, 0..all_quads.len() as u32);
            }
        }

        // Draw cursor circle using SDF pipeline.
        {
            let cursor_prim = Sdf2dPrim::circle(
                cursor_world,
                cursor_radius_world,
                Mat2::IDENTITY.to_cols_array(),
                Vec4::new(0.4, 1.0, 0.6, 1.0),
                Vec4::new(0.4, 1.0, 0.6, 1.0),
            );
            self.queue.write_buffer(
                &self.overlay_buffer,
                0,
                bytemuck::bytes_of(&crate::common_state::SdfHeader::new(1)),
            );
            self.queue.write_buffer(
                &self.overlay_buffer,
                crate::common_state::SDF_HEADER_SIZE as u64,
                bytemuck::cast_slice(&[cursor_prim]),
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Cursor"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sdf_pipeline);
            pass.set_bind_group(0, &self.overlay_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        self.queue.present(surface_texture);
    }

    /*/// Generate an MTSDF atlas from a TTF font file and switch to it.
    ///
    /// Requires the `wgpu` feature flag. Currently a placeholder — returns `None`
    /// until a font file is bundled.
    #[cfg(feature = "wgpu")]
    pub fn generate_and_switch_to_mtsdf_atlas(
        &mut self,
        font_path: &std::path::Path,
        config: &MtsdfAtlasConfig,
    ) -> Option<()> {
        let result = generate_mtsdf_atlas(font_path, config, &self.device, &self.queue)?;

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some("MTSDF Text Atlas"),
                size: wgpu::Extent3d {
                    width: result.dimensions.0,
                    height: result.dimensions.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::default(),
            &result.image,
        );
        let view = texture.create_view(&Default::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("MTSDF Atlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        self.text_atlas_texture = texture;
        self.text_atlas_view = view;
        self.text_atlas_sampler = sampler;
        self.text_atlas_width = result.dimensions.0;
        self.text_atlas_height = result.dimensions.1;
        self.text_atlas_data = result.data;
        self.text_atlas_type = TextAtlasType::Mtsdf;

        // Recreate bind group with new texture/sampler.
        self.text_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MSDF Text Bind Group"),
            layout: &self.text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.text_quads_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.text_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.text_atlas_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.text_globals_buffer.as_entire_binding(),
                },
            ],
        });

        Some(())
    }

    /// Non-wgpu stub: generation not available without the `wgpu` feature.
    #[cfg(not(feature = "wgpu"))]
    pub fn generate_and_switch_to_mtsdf_atlas(
        &mut self,
        _font_path: &std::path::Path,
        _config: &MtsdfAtlasConfig,
    ) -> Option<()> {
        None
    }*/
}
