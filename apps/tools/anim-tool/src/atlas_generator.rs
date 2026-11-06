//! MTSDF (Multi-channel Signed Distance Field) atlas generator.
//!
//! Uses `klyff_msdf` with the `wgpu` feature for GPU-accelerated atlas generation.
//! Generates MTSDF atlases where the SDF is stored in the alpha channel.
//!
//! Uses bundled TTF font files under `font_files/`.

use crate::render::TextQuad;
use image::RgbaImage;
use klyff_msdf::{
    skrifa::{FontRef, GlyphId, MetadataProvider},
    GpuAtlasRegion, MsdfGenerator, MtsdfGpuWriter, SkrifaOutlineProvider,
};
use std::{collections::HashMap, path::Path};

pub type MtsdfGlyphs = HashMap<(FontId, char), GlyphInfo>;

/// Everything the renderer needs to draw one glyph.
#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    /// Atlas UVs, already normalized to [0,1].
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Quad size in pixels at the reference em size (before scaling).
    pub quad_size: [f32; 2],
    /// Offset from pen origin to the top-left of the quad, in pixels.
    pub bearing: [f32; 2],
    /// How far to advance the pen after this glyph, in pixels.
    pub advance: f32,
}

/// A font atlas with glyph metadata for rendering.
pub struct FontAtlas {
    pub em_size: f32,
    pub size: u32,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub glyphs: MtsdfGlyphs,
}

/// Font identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontId {
    Normal,
    Bold,
}

/// A simple first-fit packer for the atlas.
pub struct SimplePacker {
    atlas_width: u32,
    atlas_height: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    padding: u32,
    pub peak_y: u32,
}

impl SimplePacker {
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            atlas_width: width,
            atlas_height: height,
            cursor_x: padding,
            cursor_y: padding,
            row_height: 0,
            padding,
            peak_y: 0,
        }
    }

    /// Returns (x, y) of the top-left of the allocated cell, including padding.
    /// The content rect is (x+padding, y+padding, w-2p, h-2p).
    pub fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.cursor_x + w + self.padding > self.atlas_width {
            self.cursor_x = self.padding;
            self.cursor_y += self.row_height + self.padding;
            self.row_height = 0;
        }
        if self.cursor_y + h + self.padding > self.atlas_height {
            return None;
        }
        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + self.padding;
        self.row_height = self.row_height.max(h);
        self.peak_y = self.peak_y.max(self.cursor_y + h);
        Some(pos)
    }
}

/// Configuration for atlas generation.
pub struct AtlasGenConfig {
    pub atlas_size: u32,
    pub em_size: f32,
    pub padding: u32,
}

impl Default for AtlasGenConfig {
    fn default() -> Self {
        Self {
            atlas_size: 1024,
            em_size: 48.0,
            padding: 2,
        }
    }
}

/// Build an MTSDF atlas from font files using the GPU.
pub fn build_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fonts: &[(FontId, &Path)],
    chars: &[char],
    config: &AtlasGenConfig,
) -> FontAtlas {
    let atlas_size = config.atlas_size;
    let em_size = config.em_size;
    let padding = config.padding;

    // Create the atlas texture (linear, not sRGB - raw distance data).
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MTSDF Atlas"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut packer = SimplePacker::new(atlas_size, atlas_size, padding);
    let mut glyphs: HashMap<(FontId, char), GlyphInfo> = HashMap::new();

    let mut generator = MsdfGenerator::new();
    let mut writer = MtsdfGpuWriter::new(device);

    for (font_id, font_path) in fonts {
        let font_data = match std::fs::read(font_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to read font {:?}: {}", font_path, e);
                continue;
            }
        };
        let font = match FontRef::from_index(&font_data, 0) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to parse font {:?}: {}", font_path, e);
                continue;
            }
        };

        // Pre-compute all metrics first (avoids borrow conflicts with outline reading).
        let mut glyph_data: Vec<(
            char,
            GlyphId,
            f32,
            klyff_msdf::skrifa::raw::types::BoundingBox<f32>,
        )> = Vec::new();
        for &ch in chars {
            let glyph_id = match font.charmap().map(ch) {
                Some(g) if g.to_u32() != 0 => g,
                _ => continue,
            };
            let metrics = font.glyph_metrics(
                klyff_msdf::skrifa::instance::Size::new(em_size),
                klyff_msdf::skrifa::instance::LocationRef::default(),
            );
            let advance = metrics.advance_width(glyph_id).unwrap_or(0.0);
            let bbox = metrics.bounds(glyph_id).unwrap_or_default();
            glyph_data.push((ch, glyph_id, advance, bbox));
        }

        // Now read outlines and build regions (font borrowed immutably for outline).
        for (codepoint, glyph_id, advance, bbox) in glyph_data {
            let outline =
                match generator.read_glyph(&SkrifaOutlineProvider::new(font.clone(), glyph_id)) {
                    Ok(o) => o,
                    Err(_) => {
                        // Empty-outline glyph (space or a fallback)
                        glyphs.insert(
                            (*font_id, codepoint),
                            GlyphInfo {
                                uv_min: [0.0, 0.0],
                                uv_max: [0.0, 0.0],
                                quad_size: [0.0, 0.0],
                                bearing: [0.0, 0.0],
                                advance,
                            },
                        );
                        continue;
                    }
                };

            let content_w = ((bbox.x_max - bbox.x_min).ceil() as u32).max(1);
            let content_h = ((bbox.y_max - bbox.y_min).ceil() as u32).max(1);
                
            let cell_w = content_w + 2 * padding;
            let cell_h = content_h + 2 * padding;

            let (x, y) = match packer.allocate(cell_w, cell_h) {
                Some(p) => p,
                None => {
                    eprintln!(
                        "Atlas full ({}px). Increase size or trim charset.",
                        atlas_size
                    );
                    break;
                }
            };

            let region = GpuAtlasRegion {
                layer: 0,
                min_x: x,
                min_y: y,
                width: cell_w,
                height: cell_h,
                padding_x: padding,
                padding_y: padding,
            };

            writer.add_glyph(outline, region);

            glyphs.insert(
                (*font_id, codepoint),
                GlyphInfo {
                    uv_min: [
                        (x + padding) as f32 / atlas_size as f32,
                        (y + padding) as f32 / atlas_size as f32,
                    ],
                    uv_max: [
                        (x + padding + content_w) as f32 / atlas_size as f32,
                        (y + padding + content_h) as f32 / atlas_size as f32,
                    ],
                    quad_size: [content_w as f32, content_h as f32],
                    bearing: [bbox.x_min, -bbox.y_max],
                    advance,
                },
            );
        }
    }

    // One flush: all glyphs generated in a single GPU pass-set.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    writer.write_glyphs(device, queue, &mut encoder, &texture);
    queue.submit(std::iter::once(encoder.finish()));

    eprintln!(
        "Atlas used {} / {} px vertically",
        packer.peak_y, atlas_size
    );

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("MTSDF Atlas Sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    FontAtlas {
        em_size,
        size: atlas_size,
        texture,
        view,
        sampler,
        glyphs,
    }
}

/// Read a wgpu texture back to CPU as an `RgbaImage`.
pub fn read_texture_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Option<RgbaImage> {
    let unpadded_bpr = 4 * width;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
    let padded_bpr = unpadded_bpr.div_ceil(align) * align;

    // Create a readback buffer.
    let buffer_size = (padded_bpr as u64) * (height as u64);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Atlas Readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    // Map and wait.
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });

    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    // Surface any error
    rx.recv().ok()?.ok()?;

    // Copy data out
    let data = slice.get_mapped_range().ok()?;
    let mut image = RgbaImage::new(width, height);
    let row_bytes = unpadded_bpr as usize;
    let stride = padded_bpr as usize;
    for y in 0..height as usize {
        let src = &data[y * stride..y * stride + row_bytes];
        let dst_start = y * row_bytes;
        image.as_mut()[dst_start..dst_start + row_bytes].copy_from_slice(src);
    }
    drop(data);
    readback.unmap();

    Some(image)
}

/// Save the rendered atlas texture as a PNG file for debugging.
pub fn save_atlas_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let image = read_texture_to_image(device, queue, texture, width, height)
        .ok_or("Failed to read atlas texture from GPU")?;
    image.save_with_format(output_path, image::ImageFormat::Png)?;
    Ok(())
}

/// Generate `GlyphInstance`s from a text string using the atlas.
pub fn layout_text_instances(
    text: &str,
    font_id: FontId,
    atlas: &FontAtlas,
    origin: [f32; 2],
    px_size: f32,
    em_size: f32,
    out: &mut Vec<GlyphInstance>,
) {
    let scale = px_size / em_size;
    let mut pen = origin;

    for ch in text.chars() {
        match atlas.glyphs.get(&(font_id, ch)) {
            Some(g) => {
                let top_left = [pen[0] + g.bearing[0] * scale, pen[1] + g.bearing[1] * scale];

                out.push(GlyphInstance {
                    pos_px: top_left,
                    size_px: [g.quad_size[0] * scale, g.quad_size[1] * scale],
                    uv_min: g.uv_min,
                    uv_max: g.uv_max,
                });
                pen[0] += g.advance * scale;
            }
            None => {
                pen[0] += 0.25 * scale; // default space advance
            }
        }
    }
}

pub fn instances_to_quads(
    instances: &[GlyphInstance],
    screen_w: f32,
    screen_h: f32,
    color: [f32; 4],
    out: &mut Vec<TextQuad>,
) {
    out.clear();
    for gi in instances {
        let min = [
            gi.pos_px[0] / screen_w * 2. - 1.,
            gi.pos_px[1] / screen_h * 2. - 1.,
        ];
        let max = [
            (gi.pos_px[0] + gi.size_px[0]) / screen_w * 2. - 1.,
            (gi.pos_px[1] + gi.size_px[1]) / screen_h * 2. - 1.,
        ];
        out.push(TextQuad {
            min,
            max,
            uv_min: gi.uv_min,
            uv_max: gi.uv_max,
            color,
        });
    }
}

/// A single rendered glyph instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphInstance {
    pub pos_px: [f32; 2],
    pub size_px: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packer_basic() {
        let mut packer = SimplePacker::new(100, 100, 2);
        assert_eq!(packer.allocate(10, 10), Some((2, 2)));
        assert_eq!(packer.allocate(20, 15), Some((14, 2)));
        assert!(packer.peak_y > 0);
    }

    #[test]
    fn test_packer_wrap() {
        let mut packer = SimplePacker::new(50, 100, 2);
        assert_eq!(packer.allocate(30, 20), Some((2, 2)));
        assert_eq!(packer.allocate(30, 20), Some((2, 24)));
    }
}
