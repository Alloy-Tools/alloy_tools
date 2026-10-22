use crate::app::FPS;
use al_math::{
    matrix::Mat2,
    sdf::{Sdf2dPrim, SDF_PRIM_SIZE},
    transform::Transform2d,
    vec::{Vec2, Vec4},
};
use al_skeleton::skeleton::ColliderRef;
use bytemuck::Zeroable;
use wgpu::{Surface, SurfaceConfiguration};

pub const MAX_SDF_PRIMS: usize = 1024;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, Zeroable)]
pub struct Globals {
    pub resolution: [f32; 2],
    pub camera_pos: [f32; 2],
    pub cursor_world: [f32; 2],
    pub view_size: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, Zeroable)]
pub struct SdfHeader {
    pub count: u32,
    pub _pad: [u32; 3],
}

impl SdfHeader {
    pub fn new(count: u32) -> Self {
        Self {
            count,
            _pad: [0; 3],
        }
    }
}

pub const SDF_HEADER_SIZE: usize = std::mem::size_of::<SdfHeader>();

#[allow(unused)]
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
    pub globals_buffer: wgpu::Buffer,
    pub union_buffer: wgpu::Buffer,
    pub union_bind_group: wgpu::BindGroup,
    pub subtract_buffer: wgpu::Buffer,
    pub subtract_bind_group: wgpu::BindGroup,
    pub overlay_buffer: wgpu::Buffer,
    pub overlay_bind_group: wgpu::BindGroup,
    pub bg_pipeline: wgpu::RenderPipeline,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("designer_sdf.wgsl").into()),
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
            globals_buffer,
            union_buffer,
            union_bind_group,
            subtract_buffer,
            subtract_bind_group,
            overlay_buffer,
            overlay_bind_group,
            bg_pipeline,
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
        }
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

    pub fn render_designer(&mut self, d: &mut crate::state::DesignerState) {
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
        let globals = Globals {
            resolution,
            camera_pos: d.camera.pos,
            cursor_world: [d.cursor_world.x, d.cursor_world.y],
            view_size: d.camera.view_size,
            _pad: 0.,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

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
                    crate::state::OnionMode::Keys => {
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
                    crate::state::OnionMode::Time => {
                        for k in 1..=d.onion_keys {
                            let dt = k as f32 / FPS;
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
                let color = if d.extra_active.get(i).copied().unwrap_or(false) {
                    Vec4::new(0.3, 0.95, 0.3, 1.)
                } else {
                    Vec4::new(0.3, 0.55, 0.3, 0.35)
                };
                overlay_prims.push(if let Some(idx) = extra.parent() {
                    shape.to_prim(world[idx], color)
                } else {
                    shape.to_prim(Transform2d::IDENTITY, color)
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
            ));
        }

        // Cursor marker so you can see what you're pointing at.
        overlay_prims.push(Sdf2dPrim::circle(
            d.cursor_world,
            0.006,
            Mat2::IDENTITY.to_cols_array(),
            Vec4::new(0.4, 1., 0.6, 1.),
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

        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        self.queue.present(surface_texture);
    }
}

fn write_sdf(queue: &wgpu::Queue, buffer: &wgpu::Buffer, prims: &[Sdf2dPrim]) {
    debug_assert!(
        prims.len() <= MAX_SDF_PRIMS,
        "prim overflow: {} > {}",
        prims.len(),
        MAX_SDF_PRIMS,
    );
    let n = prims.len().min(MAX_SDF_PRIMS);
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&SdfHeader::new(n as u32)));
    if n > 0 {
        queue.write_buffer(
            buffer,
            SDF_HEADER_SIZE as u64,
            bytemuck::cast_slice(&prims[..n]),
        );
    }
}

fn push_body_prims(
    d: &crate::state::DesignerState,
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
            ),
        ));
    }

    for bone in bones {
        let depth = bone.depth();
        let col = depth_tint(base_color, depth);
        let o_world = world[bone.origin_index()];
        let m_inv = o_world.linear.try_inverse().unwrap_or(Mat2::IDENTITY);
        let wo = o_world.transform_point(Vec2::ZERO);
        let wt = world[bone.tip_index()].transform_point(Vec2::ZERO);
        let b_local = m_inv * (wt - wo);
        batch.push((
            depth,
            Sdf2dPrim::capsule(
                wo,
                b_local,
                bone.radius(),
                m_inv.to_cols_array(),
                Vec4::new(col.x, col.y, col.z, col.w * alpha_mul),
            ),
        ));
    }

    batch.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    prims.extend(batch.into_iter().map(|(_, p)| p));
}
