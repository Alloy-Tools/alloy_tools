use crate::{
    app::{MenuAction, MenuScreen, MenuState},
    map::MAP_RES,
};
use bytemuck::Zeroable;
use wgpu::{Surface, SurfaceConfiguration};
use winit::window::Window;

const MAX_MENU_SLOTS: usize = 16;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MenuSlot {
    pub rect: [f32; 4],
    pub params: [f32; 4],
    pub fill: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MenuUI {
    pub count: u32,
    pub _pad: [u32; 3],
    pub slots: [MenuSlot; MAX_MENU_SLOTS],
}

pub struct CapsuleRect {
    pub center_x: f32,
    pub center_y: f32,
    pub half_w: f32,
    pub half_h: f32,
    pub selected: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub resolution: [f32; 2],
    pub camera_pos: [f32; 2],
    pub cursor_world: [f32; 2],
    pub view_size: f32,
    pub _pad: f32,
}

pub struct RenderState {
    pub surface: Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub globals_buffer: wgpu::Buffer,
    pub player_buffer: wgpu::Buffer,
    pub projectile_buffer: wgpu::Buffer,
    // Kept alive so the bind group's texture resource isn't dropped.
    #[allow(dead_code)]
    map_view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    // Glyphon
    pub font_system: glyphon::FontSystem,
    pub swash_cache: glyphon::SwashCache,
    pub _cache: glyphon::Cache,
    pub atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,
    pub viewport: glyphon::Viewport,
    // Menu
    pub menu_pipeline: wgpu::RenderPipeline,
    pub menu_buffer: wgpu::Buffer,
    pub menu_bind_group: wgpu::BindGroup,
}

impl RenderState {
    pub async fn new(window: std::sync::Arc<Window>) -> Self {
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

        // ----- Shader -----
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SDF Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // ----- Map Texture -----
        let map_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Map SDF Texture"),
            size: wgpu::Extent3d {
                width: MAP_RES,
                height: MAP_RES,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let map_data = crate::map::MapData::new(crate::map::map_sdf);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &map_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(map_data.sdf_map()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(MAP_RES * 2),
                rows_per_image: Some(MAP_RES),
            },
            wgpu::Extent3d {
                width: MAP_RES,
                height: MAP_RES,
                depth_or_array_layers: 1,
            },
        );
        let map_view = map_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let map_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Map Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // ----- Buffers -----
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Buffer"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let player_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Player Buffer"),
            size: std::mem::size_of::<crate::game::PlayerData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let projectile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projectile Buffer"),
            size: std::mem::size_of::<crate::game::ProjectileData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ----- Bind buffer group layout -----
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: player_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&map_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&map_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: projectile_buffer.as_entire_binding(),
                },
            ],
        });

        // ----- Pipeline -----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("fs_main"),
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

        // Menu
        let menu_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Menu Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("menu.wgsl").into()),
        });

        let menu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Menu Slot Buffer"),
            size: std::mem::size_of::<MenuUI>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let menu_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Menu Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let menu_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Menu Bind Group"),
            layout: &menu_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: menu_buffer.as_entire_binding(),
            }],
        });

        let menu_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Menu Pipeline Layout"),
            bind_group_layouts: &[Some(&menu_bgl)],
            immediate_size: 0,
        });

        let menu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Menu Pipeline"),
            layout: Some(&menu_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &menu_shader,
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
                module: &menu_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // ← key
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            globals_buffer,
            player_buffer,
            projectile_buffer,
            map_view,
            bind_group,
            font_system,
            swash_cache,
            _cache: cache,
            atlas,
            text_renderer,
            viewport,
            menu_pipeline,
            menu_buffer,
            menu_bind_group,
        }
    }

    pub fn render_game(&mut self, game: &mut crate::game::GameState) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => return,
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let resolution = [self.config.width as f32, self.config.height as f32];
        game.set_resolution(resolution);

        // Update globals for this frame
        let globals = Globals {
            resolution,
            camera_pos: game.camera.pos,
            cursor_world: game.cursor_world,
            view_size: game.camera.view_size,
            _pad: 0.,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Update player buffer
        self.queue.write_buffer(
            &self.player_buffer,
            0,
            bytemuck::bytes_of(&crate::game::PlayerData::new(game)),
        );

        // Update projectile buffer
        self.queue.write_buffer(
            &self.projectile_buffer,
            0,
            bytemuck::bytes_of(&crate::game::ProjectileData::new(game)),
        );

        let mut text_buffer =
            glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(16., 20.));
        text_buffer.set_size(
            Some(self.config.width as f32),
            Some(self.config.height as f32),
        );

        // Text Overlay
        let text_areas = build_scoreboard(&mut text_buffer, &mut self.font_system, game);

        //REVIEW: Only update on change
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        // SDF Pass
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SDF pass"),
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

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // Prepare text
        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        // Overlay Pass
        {
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

            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .unwrap();
        }

        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        self.queue.present(surface_texture);
    }

    pub fn render_menu(&mut self, menu: &mut MenuState) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => return,
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let window = (self.config.width as f32, self.config.height as f32);

        let mut text_buffer =
            glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(32., 64.));
        text_buffer.set_size(Some(window.0), Some(window.1));

        // Text Overlay
        let (text_areas, capsules) =
            build_menu(menu, window, &mut text_buffer, &mut self.font_system);

        let mut ui = MenuUI::zeroed();
        let n = capsules.len().min(MAX_MENU_SLOTS);
        ui.count = n as u32;
        for (i, cap) in capsules.iter().take(MAX_MENU_SLOTS).enumerate() {
            let t = cap.selected;
            let fill = [
                0.11 + (0.20 - 0.11) * t,
                0.13 + (0.24 - 0.13) * t,
                0.18 + (0.32 - 0.18) * t,
                0.70 + (0.85 - 0.70) * t,
            ];
            ui.slots[i] = MenuSlot {
                rect: [cap.center_x, cap.center_y, cap.half_w, cap.half_h],
                params: [10., 2., cap.selected, 0.],
                fill,
            };
        }
        self.queue
            .write_buffer(&self.menu_buffer, 0, bytemuck::bytes_of(&ui));

        //REVIEW: Only update on change
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
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Menu Capsule Pass"),
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
            pass.set_pipeline(&self.menu_pipeline);
            pass.set_bind_group(0, &self.menu_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Menu Text Pass"),
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

fn build_scoreboard<'a>(
    buffer: &'a mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
    game: &mut crate::game::GameState,
) -> Vec<glyphon::TextArea<'a>> {
    let mut text = String::new();
    let me = game.server_id;

    if game.scoreboard.entries().is_empty() {
        text.push_str("Waiting for players...");
    } else {
        let mut entries = game.scoreboard.entries().to_vec();
        entries.sort_by(|a, b| b.kills.cmp(&a.kills));

        for e in &entries {
            let marker = if Some(e.conn_id) == me { ">" } else { " " };
            text.push_str(&format!(
                "{} c{:<2}   {:>2}/{:<2}\n",
                marker, e.conn_id, e.kills, e.deaths
            ));
        }
    }

    buffer.set_text(
        &text,
        &glyphon::Attrs::new().family(glyphon::Family::Monospace),
        glyphon::Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    vec![glyphon::TextArea {
        buffer,
        left: 12.,
        top: 12.,
        scale: 1.,
        bounds: glyphon::TextBounds {
            left: 0,
            top: 0,
            right: 400,
            bottom: 400,
        },
        default_color: glyphon::Color::rgb(230, 230, 240),
        custom_glyphs: &[],
    }]
}

fn centered_text_area<'a>(
    buffer: &'a mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
    text: &str,
    center_lines: bool,
    items: Option<&[MenuAction]>,
    anim: Option<&[f32]>,
    selected: usize,
    attrs: glyphon::Attrs,
    scale: f32,
    window: (f32, f32),
    color: glyphon::Color,
) -> (Vec<glyphon::TextArea<'a>>, Vec<CapsuleRect>) {
    buffer.set_text(
        text,
        &attrs,
        glyphon::Shaping::Advanced,
        if center_lines {
            Some(glyphon::cosmic_text::Align::Center)
        } else {
            None
        },
    );
    buffer.shape_until_scroll(font_system, false);

    let mut max_w = 0.0f32;
    let mut total_h = 0.0f32;
    let runs = buffer
        .layout_runs()
        .map(|r| (r.line_top, r.line_height, r.line_w))
        .collect::<Vec<_>>();
    for (_, h, w) in &runs {
        max_w = max_w.max(*w);
        total_h += *h;
    }

    let top = ((window.1 - total_h * scale) * 0.5).max(0.);
    let left = if center_lines {
        0.
    } else {
        ((window.0 - max_w * scale) * 0.5).max(0.)
    };

    let areas = vec![glyphon::TextArea {
        buffer,
        left,
        top,
        scale,
        bounds: glyphon::TextBounds {
            left: 0,
            top: 0,
            right: window.0 as i32,
            bottom: window.1 as i32,
        },
        default_color: color,
        custom_glyphs: &[],
    }];

    let capsules = if let Some(items) = items {
        let center_x = if center_lines {
            window.0 * 0.5
        } else {
            left + max_w * scale * 0.5
        };
        let n_items = items.len();
        let non_empty = runs.iter().filter(|(_, _, w)| *w > 0.1).collect::<Vec<_>>();
        let item_runs = &non_empty[non_empty.len().saturating_sub(n_items)..];

        let half_w = max_w * scale * 0.5 + 24.;

        let mut capsules = Vec::with_capacity(n_items);
        for (i, (line_y, line_h, _)) in item_runs.iter().enumerate() {
            let line_top = top + line_y * scale;
            let line_bottom = line_top + line_h * scale;
            //let half_h = (line_bottom - line_top) * 0.5 + 6.;
            let half_h = line_h * scale * 0.4;
            capsules.push(CapsuleRect {
                center_x,
                center_y: (line_top + line_bottom) * 0.5,
                half_w,
                half_h,
                selected: anim
                    .map(|a| a.get(i).copied().unwrap_or(0.))
                    .unwrap_or(if i == selected { 1. } else { 0. }),
            });
        }
        capsules
    } else {
        vec![]
    };

    (areas, capsules)
}

fn build_menu<'a>(
    menu: &mut MenuState,
    window: (f32, f32),
    buffer: &'a mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
) -> (Vec<glyphon::TextArea<'a>>, Vec<CapsuleRect>) {
    if let Some(screen) = menu.screen() {
        match screen {
            crate::app::MenuScreen::Main { .. } => {
                build_main_menu(menu, window, buffer, font_system)
            }
            crate::app::MenuScreen::ClientAddr { .. } => {
                build_client_addr(menu, window, buffer, font_system)
            }
        }
    } else {
        (vec![], vec![])
    }
}

fn build_main_menu<'a>(
    menu: &mut MenuState,
    window: (f32, f32),
    buffer: &'a mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
) -> (Vec<glyphon::TextArea<'a>>, Vec<CapsuleRect>) {
    let Some(MenuScreen::Main {
        selected,
        items,
        anim,
        ..
    }) = menu.screen()
    else {
        return (vec![], vec![]);
    };
    let mut text = String::new();
    text.push_str("SDF Game\n\n");
    for item in items {
        let label = match item {
            MenuAction::Client { .. } => "Connect to server",
            MenuAction::Host => "Host and join",
            MenuAction::Quit => "Quit",
        };
        text.push_str(&format!("{label}\n"));
    }

    centered_text_area(
        buffer,
        font_system,
        &text,
        true,
        Some(&items),
        Some(anim),
        *selected,
        glyphon::Attrs::new().family(glyphon::Family::SansSerif),
        1.,
        window,
        glyphon::Color::rgb(240, 240, 250),
    )
}

fn build_client_addr<'a>(
    menu: &mut MenuState,
    window: (f32, f32),
    buffer: &'a mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
) -> (Vec<glyphon::TextArea<'a>>, Vec<CapsuleRect>) {
    let Some(MenuScreen::ClientAddr { addr_input, blink }) = menu.screen_mut() else {
        return (vec![], vec![]);
    };
    let text = format!(
        "Connect to server\n\n\
         > {}{}\n\n\
         Enter to connect  •  Esc to go back",
        addr_input,
        if blink.elapsed().as_millis() % 1000 < 500 {
            "▌"
        } else {
            " "
        }
    );

    centered_text_area(
        buffer,
        font_system,
        &text,
        false,
        None,
        None,
        0,
        glyphon::Attrs::new().family(glyphon::Family::Monospace),
        1.,
        window,
        glyphon::Color::rgb(240, 240, 250),
    )
}
