use al_math::vec::{Vec2, Vec4};
use al_skeleton::skeleton::ColliderRef;
use bytemuck::Zeroable;
use wgpu::{Surface, SurfaceConfiguration};

pub const MAX_SKELETON_PRIMS: usize = 256;

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
#[derive(Clone, Copy, Debug, bytemuck::Pod, Zeroable)]
pub struct SdfPrim {
    pub origin: Vec2,
    pub tip: Vec2,
    pub radius: f32,
    pub kind: u32,
    pub _pad: Vec2,
    pub color: Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, Zeroable)]
pub struct SdfData {
    pub count: u32,
    pub _pad: [u32; 3],
    pub prims: [SdfPrim; MAX_SKELETON_PRIMS],
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
    pub globals_buffer: wgpu::Buffer,
    pub rig_pipeline: wgpu::RenderPipeline,
    pub rig_buffer: wgpu::Buffer,
    pub rig_bind_group: wgpu::BindGroup,
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

        let designer_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Designer Buffer"),
            size: std::mem::size_of::<SdfData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let designer_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Designer BGL"),
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
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let designer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Designer BG"),
            layout: &designer_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: designer_buffer.as_entire_binding(),
                },
            ],
        });

        let designer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SDF Designer Layout"),
                bind_group_layouts: &[Some(&designer_bgl)],
                immediate_size: 0,
            });

        let designer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Designer Pipeline"),
            layout: Some(&designer_pipeline_layout),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

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
            globals_buffer,
            rig_pipeline: designer_pipeline,
            rig_buffer: designer_buffer,
            rig_bind_group: designer_bind_group,
            font_system,
            swash_cache,
            _cache: cache,
            atlas,
            text_renderer,
            viewport,
        }
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
        let world = d.skeleton.world_transforms(&d.pose);
        let mut prims: Vec<SdfPrim> = Vec::with_capacity(MAX_SKELETON_PRIMS);

        // Body fills first (bottom layer).
        for (i, joint) in d.skeleton.joints().iter().enumerate() {
            if joint.radius() <= 0.0 {
                continue;
            }
            let c = world[i].transform_point(Vec2::ZERO);
            let base_color = Vec4::new(0.82, 0.85, 0.92, 1.);
            prims.push(SdfPrim {
                origin: Vec2::new(c.x, c.y),
                tip: Vec2::new(c.x, c.y),
                radius: joint.radius(),
                kind: 0,
                _pad: Vec2::new(0., 0.),
                color: depth_tint(base_color, joint.depth()),
            });
        }
        for bone in d.skeleton.bones() {
            let a = world[bone.origin_index()].transform_point(Vec2::ZERO);
            let e = world[bone.tip_index()].transform_point(Vec2::ZERO);
            let base_color = Vec4::new(0.82, 0.85, 0.92, 1.);
            prims.push(SdfPrim {
                origin: Vec2::new(a.x, a.y),
                tip: Vec2::new(e.x, e.y),
                radius: bone.radius(),
                kind: 1,
                _pad: Vec2::new(0., 0.),
                color: depth_tint(base_color, bone.depth()),
            });
        }

        if d.show_extras {
            for (i, extra) in d.skeleton.extras().iter().enumerate() {
                let active = d.extra_active.get(i).copied().unwrap_or(false);
                let (o, t, r) = match extra.shape() {
                    al_skeleton::collider::Shape2d::Circle { origin, radius } => {
                        let o_offset = origin.to_components().0;
                        let o = if let Some(idx) = extra.parent() {
                            world[idx].transform_point(o_offset)
                        } else {
                            o_offset
                        };
                        (o, o, radius)
                    }
                    al_skeleton::collider::Shape2d::Capsule {
                        origin,
                        tip,
                        radius,
                    } => {
                        let o_offset = origin.to_components().0;
                        let t_offset = tip.to_components().0;
                        let (o, t) = if let Some(idx) = extra.parent() {
                            (
                                world[idx].transform_point(o_offset),
                                world[idx].transform_point(t_offset),
                            )
                        } else {
                            (o_offset, t_offset)
                        };
                        (o, t, radius)
                    }
                };
                prims.push(SdfPrim {
                    origin: o,
                    tip: t,
                    radius: r,
                    kind: al_skeleton::collider_kind::ColliderKind::HIT.inner() as u32,
                    _pad: Vec2::ZERO,
                    color: if active {
                        Vec4::new(0.3, 0.95, 0.3, 1.)
                    } else {
                        Vec4::new(0.3, 0.55, 0.3, 0.35)
                    },
                });
            }
        }

        // Joint markers on top — small dots, selected joint highlighted.
        for (i, _) in d.skeleton.joints().iter().enumerate() {
            let c = world[i].transform_point(Vec2::ZERO);
            let selected = d.selected == Some(ColliderRef::Joint(i));
            prims.push(SdfPrim {
                origin: Vec2::new(c.x, c.y),
                tip: Vec2::new(c.x, c.y),
                radius: if selected { 0.016 } else { 0.008 },
                kind: 0,
                _pad: Vec2::new(0., 0.),
                color: if selected {
                    Vec4::new(1., 0.75, 0.15, 1.)
                } else {
                    Vec4::new(1., 1., 1., 0.85)
                },
            });
        }

        // Cursor marker so you can see what you're pointing at.
        prims.push(SdfPrim {
            origin: Vec2::new(d.cursor_world.x, d.cursor_world.y),
            tip: Vec2::new(d.cursor_world.x, d.cursor_world.y),
            radius: 0.006,
            kind: 0,
            _pad: Vec2::new(0., 0.),
            color: Vec4::new(0.4, 1., 0.6, 1.),
        });

        let n = prims.len().min(MAX_SKELETON_PRIMS);
        let mut data = SdfData::zeroed();
        data.count = n as u32;
        data.prims[..n].copy_from_slice(&prims[..n]);
        self.queue
            .write_buffer(&self.rig_buffer, 0, bytemuck::bytes_of(&data));

        let mut encoder = self.device.create_command_encoder(&Default::default());
        // ---- Skeleton Pass (instanced quads, alpha-blended) ----
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Designer Rig Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.rig_pipeline);
            pass.set_bind_group(0, &self.rig_bind_group, &[]);
            pass.draw(0..6, 0..n as u32);
        }

        // ---- Text HUD ----
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

        {
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
