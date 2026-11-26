use crate::text::{
    atlas_state::AtlasState,
    common::{AtlasHandle, ShapedText, TextEffect, TextQuad, TextStyle},
};

#[derive(Debug, Default, Clone)]
pub struct TextBatcher {
    groups: Vec<(AtlasHandle, Vec<TextQuad>, Vec<TextEffect>)>,
    total: usize,
}

impl TextBatcher {
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            total: 0,
        }
    }

    pub fn clear(&mut self) {
        self.groups.clear();
        self.total = 0;
    }

    pub fn quad_count(&self) -> usize {
        self.total
    }

    pub fn group_for(&mut self, atlas: AtlasHandle) -> (&mut Vec<TextQuad>, &mut Vec<TextEffect>) {
        let idx = if let Some(idx) = self.groups.iter().position(|(h, _, _)| *h == atlas) {
            idx
        } else {
            self.groups.push((atlas, Vec::new(), Vec::new()));
            self.groups.len() - 1
        };
        let group = &mut self.groups[idx];
        (&mut group.1, &mut group.2)
    }

    pub fn queue(&mut self, shaped: &ShapedText, position: [f32; 2], style: &TextStyle) {
        let (quads, effects) = self.group_for(shaped.atlas());
        for (q, e) in shaped.quads().iter().zip(shaped.effects().iter()) {
            let mut nq = *q;
            nq.min_mut(|min| {
                min[0] += position[0];
                min[1] += position[1];
            });
            nq.max_mut(|max| {
                max[0] += position[0];
                max[1] += position[1];
            });
            nq.baseline_y_mut(|by| by + position[1]);
            nq.midline_y_mut(|my| my + position[1]);
            nq.color_mut(|col| *col = style.color());
            nq.effects_mut(|_| style.effects());
            quads.push(nq);
            effects.push(e.from_style(*style));
        }
        self.total += shaped.quads().len();
    }
}

#[derive(Debug, Clone)]
pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    quads_buffer: wgpu::Buffer,
    effects_buffer: wgpu::Buffer,
    quads_capacity: usize,
    batcher: TextBatcher,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        quad_buffer_capacity: usize,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MTSDF Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sdf_text.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text BGL"),
            entries: &[
                // quads storage
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // atlas texture
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
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // per-atlas globals
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
                // effects storage
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&pipeline_layout),
            primitive: wgpu::PrimitiveState::default(),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_text"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_effect_text"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
        });

        let quads_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Text Quads"),
            size: (quad_buffer_capacity * std::mem::size_of::<TextQuad>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let effects_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Text Effects"),
            size: (quad_buffer_capacity * std::mem::size_of::<TextEffect>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            quads_buffer,
            effects_buffer,
            quads_capacity: quad_buffer_capacity,
            batcher: TextBatcher::new(),
        }
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn quads_buffer(&self) -> &wgpu::Buffer {
        &self.quads_buffer
    }

    pub fn effects_buffer(&self) -> &wgpu::Buffer {
        &self.effects_buffer
    }

    pub fn quads_capacity(&self) -> usize {
        self.quads_capacity
    }

    pub fn batcher(&self) -> &TextBatcher {
        &self.batcher
    }

    pub fn batcher_mut(&mut self) -> &mut TextBatcher {
        &mut self.batcher
    }

    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        atlases: &AtlasState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if self.batcher.quad_count() == 0 {
            return;
        }

        let mut flat_quads = Vec::with_capacity(self.batcher.quad_count().min(self.quads_capacity));
        let mut flat_effects = Vec::with_capacity(flat_quads.capacity());
        let mut ranges = Vec::new();
        for (handle, quads, effects) in std::mem::take(&mut self.batcher.groups) {
            let start = flat_quads.len();
            let take = (self.quads_capacity - start).min(quads.len());
            if take == 0 {
                continue;
            }
            flat_quads.extend_from_slice(&quads[..take]);
            flat_effects.extend_from_slice(&effects[..take]);
            ranges.push((handle, start as u32..(start + take) as u32))
        }
        self.batcher.clear();
        if ranges.is_empty() {
            return;
        }

        queue.write_buffer(&self.quads_buffer, 0, bytemuck::cast_slice(&flat_quads));
        queue.write_buffer(&self.effects_buffer, 0, bytemuck::cast_slice(&flat_effects));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        pass.set_pipeline(&self.pipeline);
        for (handle, range) in ranges {
            let Some(atlas) = atlases.get(handle) else {
                continue;
            };
            pass.set_bind_group(0, atlas.bind_group(), &[]);
            pass.draw(0..6, range);
        }
    }
}
