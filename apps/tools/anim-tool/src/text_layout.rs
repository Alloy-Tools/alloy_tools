use crate::{
    atlas_generator::{FontId, MtsdfGlyphs},
    render::TextQuad,
};

/// A shaped glyph ready for layout.
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    /// Unicode codepoint.
    pub codepoint: u32,
    /// Glyph ID from the font.
    pub glyph_id: u32,
    /// Advance in em units.
    pub advance_x: f32,
    // TODO: re-add when integrating TTF font shaping with rustybuzz
    // pub advance_y: f32,
    // pub offset_x: f32,
    // pub offset_y: f32,
}

/// Shapes text into glyphs using atlas data directly.
/// Maps each Unicode character to its atlas entry.
pub fn shape_text(
    atlas_glyphs: &MtsdfGlyphs,
    font_id: FontId,
    text: &str,
    em_size: f32,
) -> Vec<ShapedGlyph> {
    let mut result = Vec::new();
    for ch in text.chars() {
        if let Some(info) = atlas_glyphs.get(&(font_id, ch)) {
            result.push(ShapedGlyph {
                codepoint: ch as u32,
                glyph_id: 0, // REVIEW: Should this be removed?
                advance_x: info.advance / em_size,
            });
        } else if ch.is_whitespace() {
            result.push(ShapedGlyph {
                codepoint: ch as u32,
                glyph_id: 0,
                advance_x: 0.25,
            });
        }
    }
    result
}

/// Wrapping mode for text layout.
#[derive(Clone, Copy, Debug, Default)]
pub enum WrapMode {
    /// Wrap at character boundaries (breaks mid-word if needed).
    #[default]
    PerChar,
    /// Wrap at word boundaries (keeps whole words together).
    PerWord,
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
    /// Wrapping mode (PerChar or PerWord).
    pub wrap_mode: WrapMode,
    // TODO: re-add when adding text containers/boxes for alignment within them
    // pub align_x: f32,
    // pub align_y: f32,
}

impl LayoutOptions {
    pub fn new(font_size: f32) -> Self {
        Self {
            font_size,
            max_width: None,
            line_height: 1.2,
            wrap_mode: WrapMode::default(),
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

    pub fn with_wrap_mode(mut self, wrap_mode: WrapMode) -> Self {
        self.wrap_mode = wrap_mode;
        self
    }
}

/// A placed glyph ready for quad generation.
#[derive(Clone, Debug)]
pub struct PlacedGlyph {
    pub codepoint: u32,
    pub glyph_id: u32,
    /// Position in em units (x offset from start_x).
    pub pos_x: f32,
}

/// Lay out shaped glyphs into lines with word wrapping.
///
/// Returns a list of (glyphs, line_y) where line_y is the baseline Y in em units.
/// Wrapping moves downward (increasing Y) rather than upward.
pub fn layout_text(
    shaped: &[ShapedGlyph],
    options: &LayoutOptions,
    start_x: f32,
    start_y: f32,
) -> Vec<(Vec<PlacedGlyph>, f32)> {
    let mut lines: Vec<(Vec<PlacedGlyph>, f32)> = Vec::new();

    match options.wrap_mode {
        WrapMode::PerChar => layout_text_per_char(shaped, options, start_x, start_y, &mut lines),
        WrapMode::PerWord => layout_text_per_word(shaped, options, start_x, start_y, &mut lines),
    }

    lines
}

/// Layout with per-character wrapping (breaks mid-word if needed).
fn layout_text_per_char(
    shaped: &[ShapedGlyph],
    options: &LayoutOptions,
    start_x: f32,
    start_y: f32,
    lines: &mut Vec<(Vec<PlacedGlyph>, f32)>,
) {
    let mut current_line: Vec<PlacedGlyph> = Vec::new();
    let mut current_x = start_x;
    let mut current_y = start_y;

    for glyph in shaped.iter() {
        let glyph_width = glyph.advance_x * options.font_size;

        // Check if this glyph would exceed max width
        if let Some(max_w) = options.max_width {
            let right_boundary = start_x + max_w; // Right edge = start + available width
            if current_x + glyph_width > right_boundary && !current_line.is_empty() {
                // Start new line
                lines.push((std::mem::take(&mut current_line), current_y));
                current_y += options.line_height * options.font_size; // Move DOWN
                current_x = start_x; // Start new line at same x offset
            }
        }

        current_line.push(PlacedGlyph {
            codepoint: glyph.codepoint,
            glyph_id: glyph.glyph_id,
            pos_x: current_x,
        });
        current_x += glyph_width;
    }

    if !current_line.is_empty() {
        lines.push((current_line, current_y));
    }
}

/// Layout with per-word wrapping (keeps whole words together).
fn layout_text_per_word(
    shaped: &[ShapedGlyph],
    options: &LayoutOptions,
    start_x: f32,
    start_y: f32,
    lines: &mut Vec<(Vec<PlacedGlyph>, f32)>,
) {
    let mut current_line: Vec<PlacedGlyph> = Vec::new();
    let mut current_x = start_x;
    let mut current_y = start_y;

    // Accumulate current word glyphs (relative x offsets within the word).
    let mut word_rel_x: Vec<f32> = Vec::new();
    let mut word_glyphs: Vec<PlacedGlyph> = Vec::new();
    let mut word_width = 0.0; // Total advance of the word in NDC units

    let space_codepoint = 32u32;

    for glyph in shaped.iter() {
        let glyph_width = glyph.advance_x * options.font_size;

        if glyph.codepoint == space_codepoint {
            // Space character ends the current word.
            let word_total_width = word_width + glyph_width;
            let right_boundary = start_x + options.max_width.unwrap_or(f32::INFINITY);

            if current_x + word_total_width > right_boundary && !current_line.is_empty() {
                // Word doesn't fit on current line, wrap to next line.
                lines.push((std::mem::take(&mut current_line), current_y));
                current_y += options.line_height * options.font_size; // Move DOWN
                current_x = start_x;
            }

            // Place the word glyphs at current_x + their relative offsets.
            for (i, wg) in word_glyphs.drain(..).enumerate() {
                current_line.push(PlacedGlyph {
                    codepoint: wg.codepoint,
                    glyph_id: wg.glyph_id,
                    pos_x: current_x + word_rel_x[i],
                });
            }

            // Place the space itself right after the word ends.
            current_line.push(PlacedGlyph {
                codepoint: glyph.codepoint,
                glyph_id: glyph.glyph_id,
                pos_x: current_x + word_width,
            });

            // Advance past the word and its space.
            current_x += word_total_width;

            // Reset word accumulation.
            word_rel_x.clear();
            word_width = 0.0;
        } else {
            // Accumulate glyph into current word with relative offset.
            word_rel_x.push(word_width);
            word_glyphs.push(PlacedGlyph {
                codepoint: glyph.codepoint,
                glyph_id: glyph.glyph_id,
                pos_x: 0.0, // placeholder, will be set when placed
            });
            word_width += glyph_width;
        }
    }

    // Place any remaining word at end of text.
    let right_boundary = start_x + options.max_width.unwrap_or(f32::INFINITY);
    if current_x + word_width > right_boundary
        && !current_line.is_empty()
        && !word_glyphs.is_empty()
    {
        lines.push((std::mem::take(&mut current_line), current_y));
        current_y += options.line_height * options.font_size; // Move DOWN
        current_x = start_x;
    }
    for (i, wg) in word_glyphs.drain(..).enumerate() {
        current_line.push(PlacedGlyph {
            codepoint: wg.codepoint,
            glyph_id: wg.glyph_id,
            pos_x: current_x + word_rel_x[i],
        });
    }

    if !current_line.is_empty() {
        lines.push((current_line, current_y));
    }
}

/// Generate TextQuads from laid-out glyphs using atlas data.
///
/// Returns TextQuads in NDC [-1, 1] space.
pub fn glyphs_to_quads(
    lines: &[(Vec<PlacedGlyph>, f32)],
    glyphs: &MtsdfGlyphs,
    font_id: FontId,
    em_size: f32,
    scale: f32,
    color: [f32; 4],
    effects: u32,
    baseline_y_offset: f32,
    midline_y_offset: f32,
    underline_thickness_ndc: f32,
    strike_thickness_ndc: f32,
    highlight_pad_ndc: f32,
) -> Vec<TextQuad> {
    let mut quads = Vec::new();
    for (line_glyphs, line_y) in lines.iter() {
        for glyph in line_glyphs.iter() {
            let Some(ch) = char::from_u32(glyph.codepoint) else {
                continue;
            };
            let Some(info) = glyphs.get(&(font_id, ch)) else {
                continue;
            };
            if info.quad_size[0] <= 0. || info.quad_size[1] <= 0. {
                continue;
            }

            let bx = info.bearing[0] / em_size * scale;
            let by = info.bearing[1] / em_size * scale;
            let qw = info.quad_size[0] / em_size * scale;
            let qh = info.quad_size[1] / em_size * scale;

            let min_x = glyph.pos_x + bx;
            let min_y = line_y + by;

            quads.push(TextQuad {
                min: [min_x, min_y],
                max: [min_x + qw, min_y + qh],
                uv_min: info.uv_min,
                uv_max: info.uv_max,
                color,
                effects,
                baseline_y: *line_y + baseline_y_offset,
                midline_y: (*line_y - 0.2 * scale) + midline_y_offset,
                underline_thickness: underline_thickness_ndc,
                strike_thickness: strike_thickness_ndc,
                highlight_pad: highlight_pad_ndc,
                _pad: [0., 0.],
            });
        }
    }

    quads
}
