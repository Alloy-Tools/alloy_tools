//! Text layout module for MSDF text rendering.
//!
//! Uses rustybuzz for shaping (Unicode -> glyph IDs + positions)
//! and a custom layout system (left-to-right, word wrap, line height).
//!
//! Outputs TextQuad instances ready for GPU upload.

use crate::render::{MsdfGlyph, MsdfRect, MSDF_RECT_DEFAULT, TextQuad};

/// A shaped glyph ready for layout.
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    /// Unicode codepoint.
    pub codepoint: u32,
    /// Glyph ID from the font.
    pub glyph_id: u32,
    /// Advance in em units.
    pub advance_x: f32,
    /// Advance Y in em units.
    pub advance_y: f32,
    /// X offset in em units.
    pub offset_x: f32,
    /// Y offset in em units.
    pub offset_y: f32,
}

/// Shapes text into glyphs using atlas data directly.
/// Maps each Unicode character to its atlas entry.
pub fn shape_text(atlas_glyphs: &[MsdfGlyph], text: &str, _font_size: f32) -> Vec<ShapedGlyph> {
    let mut result = Vec::new();
    for ch in text.chars() {
        let codepoint = ch as u32;
        if let Some(glyph) = atlas_glyphs.iter().find(|g| g.unicode == codepoint) {
            result.push(ShapedGlyph {
                codepoint,
                glyph_id: codepoint as u32, // REVIEW: Shouldn't this be `glyph.glyph_id` as shaping might change the glyph?
                advance_x: glyph.advance,
                advance_y: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
            });
        }
    }
    result
}

/// Layout options for text rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutOptions {
    /// Font size in em units.
    pub font_size: f32,
    /// Maximum line width in em units (None = no wrapping).
    pub max_width: Option<f32>,
    /// Line height multiplier.
    pub line_height: f32,
    /// Horizontal alignment (0.0 = left, 0.5 = center, 1.0 = right).
    pub align_x: f32,
    /// Vertical alignment (0.5 = center, 1.0 = bottom).
    pub align_y: f32,
}

impl LayoutOptions {
    pub fn new(font_size: f32) -> Self {
        Self {
            font_size,
            max_width: None,
            line_height: 1.2,
            align_x: 0.0,
            align_y: 0.0,
        }
    }

    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }
}

/// A placed glyph ready for quad generation.
#[derive(Clone, Debug)]
pub struct PlacedGlyph {
    pub codepoint: u32,
    pub glyph_id: u32,
    /// Position in em units.
    pub pos_x: f32,
    pub pos_y: f32,
}

/// Lay out shaped glyphs into lines with word wrapping.
///
/// Returns a list of (glyphs, line_y) where line_y is the baseline Y in em units.
pub fn layout_text(
    shaped: &[ShapedGlyph],
    options: &LayoutOptions,
    start_x: f32,
    start_y: f32,
) -> Vec<(Vec<PlacedGlyph>, f32)> {
    let mut lines: Vec<(Vec<PlacedGlyph>, f32)> = Vec::new();

    let mut current_line: Vec<PlacedGlyph> = Vec::new();
    let mut current_x = start_x;
    let mut current_y = start_y;

    for glyph in shaped.iter() {
        let glyph_width = glyph.advance_x * options.font_size;

        // Check if this glyph would exceed max width
        if let Some(max_w) = options.max_width {
            if current_x + glyph_width > max_w && !current_line.is_empty() {
                // Start new line
                lines.push((std::mem::take(&mut current_line), current_y));
                current_y -= options.line_height * options.font_size;
                current_x = start_x; // Start new line at same x offset
            }
        }

        current_line.push(PlacedGlyph {
            codepoint: glyph.codepoint,
            glyph_id: glyph.glyph_id,
            pos_x: current_x,
            pos_y: current_y,
        });
        current_x += glyph_width;
    }

    if !current_line.is_empty() {
        lines.push((current_line, current_y));
    }

    lines
}

/// Generate TextQuads from laid-out glyphs using atlas data.
///
/// Returns TextQuads in NDC [-1, 1] space.
pub fn glyphs_to_quads(
    lines: &[(Vec<PlacedGlyph>, f32)],
    glyphs: &[MsdfGlyph],
    atlas_w: f32,
    atlas_h: f32,
    scale: f32,
    color: [f32; 4],
) -> Vec<TextQuad> {
    let mut quads = Vec::new();

    for (line_glyphs, line_y) in lines.iter() {
        for glyph in line_glyphs.iter() {
            // Find atlas data for this glyph by matching unicode codepoint
            let atlas_entry = glyphs.iter().find(|g| g.unicode == glyph.codepoint);

            let (uv_min, uv_max) = match atlas_entry {
                Some(entry) => {
                    let uv_min = [entry.atlas_bounds.left / atlas_w, 1.0 - entry.atlas_bounds.top / atlas_h];
                    let uv_max = [entry.atlas_bounds.right / atlas_w, 1.0 - entry.atlas_bounds.bottom / atlas_h];
                    (uv_min, uv_max)
                }
                None => continue, // Skip glyphs not in atlas
            };

            // Get plane_bounds for this glyph to determine its shape extents
            let plane_bounds = glyphs_entry_plane_bounds(glyphs, glyph.codepoint);

            // Align all glyphs to a common top (line_y). This ensures capital letters
            // and other glyphs sit on the same visual line rather than floating at
            // per-glyph plane_bounds.bottom offsets.
            // min_y extends downward from line_y by each glyph's height.
            let min_x = glyph.pos_x + plane_bounds.left * scale;
            let min_y = *line_y - plane_bounds.top * scale;
            let max_x = glyph.pos_x + plane_bounds.right * scale;
            let max_y = *line_y;

            quads.push(TextQuad {
                min: [min_x, min_y],
                max: [max_x, max_y],
                uv_min,
                uv_max,
                color,
            });
        }
    }

    quads
}

fn glyphs_entry_plane_bounds<'a>(glyphs: &'a [MsdfGlyph], codepoint: u32) -> &'a MsdfRect {
    glyphs
        .iter()
        .find(|g| g.unicode == codepoint)
        .map(|g| &g.plane_bounds)
        .unwrap_or(&MSDF_RECT_DEFAULT)
}