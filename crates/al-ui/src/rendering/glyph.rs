use crate::{CoordType, RendererMetrics};
use num_traits::zero;

pub type GlyphId = u32;
pub const MAX_CODEPOINT: u32 = 0x10FFFF;
pub const FIRST_CUSTOM: GlyphId = MAX_CODEPOINT + 1;

pub enum GlyphMetrics<Coord: CoordType> {
    Monospace(MonospaceMetrics),
    Proportional(ProportionalMetrics<Coord>),
    Composite(CompositeMetrics<Coord>),
    Placeholder(MonospaceMetrics),
}

impl<Coord: CoordType> GlyphMetrics<Coord> {
    /// Access to glyph id, if any.
    pub fn glyph_id(&self) -> Option<GlyphId> {
        match self {
            GlyphMetrics::Monospace(g) => Some(g.glyph_id),
            GlyphMetrics::Proportional(g) => Some(g.glyph_id),
            GlyphMetrics::Composite(_) => None, // composites don't have a single id
            GlyphMetrics::Placeholder(g) => Some(g.glyph_id),
        }
    }

    /// Width of the glyph.
    /// For monospace: uses RendererMetrics
    /// For proportional/composite: returns advance width
    pub fn width(&self, metrics: &dyn RendererMetrics<Coord = Coord>) -> Coord {
        match self {
            GlyphMetrics::Monospace(_) => metrics.char_width(),
            GlyphMetrics::Proportional(g) => g.advance,
            GlyphMetrics::Composite(g) => g.advance,
            GlyphMetrics::Placeholder(_) => metrics.char_width(),
        }
    }

    /// Height of the glyph.
    /// For monospace: uses RendererMetrics
    /// For proportional/composite: returns bounding box height
    pub fn height(&self, metrics: &dyn RendererMetrics<Coord = Coord>) -> Coord {
        match self {
            GlyphMetrics::Monospace(_) => metrics.line_height(),
            GlyphMetrics::Proportional(g) => g.bounding_box.3 - g.bounding_box.1,
            GlyphMetrics::Composite(g) => g.bounding_box.3 - g.bounding_box.1,
            GlyphMetrics::Placeholder(_) => metrics.line_height(),
        }
    }

    /// Advance width.
    pub fn advance(&self, metrics: &dyn RendererMetrics<Coord = Coord>) -> Coord {
        match self {
            GlyphMetrics::Monospace(_) => metrics.char_width(),
            GlyphMetrics::Proportional(g) => g.advance,
            GlyphMetrics::Composite(g) => g.advance,
            GlyphMetrics::Placeholder(_) => metrics.char_width(),
        }
    }

    /// Bounding box relative to the glyph's origin.
    pub fn bounding_box(
        &self,
        metrics: &dyn RendererMetrics<Coord = Coord>,
    ) -> (Coord, Coord, Coord, Coord) {
        match self {
            GlyphMetrics::Monospace(_) => {
                (zero(), zero(), metrics.char_width(), metrics.line_height())
            }
            GlyphMetrics::Proportional(g) => g.bounding_box,
            GlyphMetrics::Composite(g) => g.bounding_box,
            GlyphMetrics::Placeholder(_) => {
                (zero(), zero(), metrics.char_width(), metrics.line_height())
            }
        }
    }

    /// Whether the glyph is monospace.
    pub fn is_monospace(&self) -> bool {
        matches!(self, GlyphMetrics::Monospace(_))
    }

    /// If this is a composite glyph, return its components.
    pub fn as_composite(&self) -> Option<&[PositionedGlyph<Coord>]> {
        match self {
            GlyphMetrics::Composite(g) => Some(&g.components),
            _ => None,
        }
    }
}

/// Monospace glyph metrics - width/height come from RendererMetrics
pub struct MonospaceMetrics {
    pub glyph_id: GlyphId,
}

pub struct ProportionalMetrics<Coord: CoordType> {
    pub glyph_id: GlyphId,
    /// (x_min, y_min, x_max, y_max)
    pub bounding_box: (Coord, Coord, Coord, Coord),
    pub bearing_x: Coord,
    pub bearing_y: Coord,
    pub advance: Coord,
}

pub struct CompositeMetrics<Coord: CoordType> {
    pub advance: Coord,
    /// (x_min, y_min, x_max, y_max)
    pub bounding_box: (Coord, Coord, Coord, Coord),
    pub components: Vec<PositionedGlyph<Coord>>,
}

pub struct PositionedGlyph<Coord: CoordType> {
    pub metrics: GlyphMetrics<Coord>,
    pub offset_x: Coord,
    pub offset_y: Coord,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_codepoint() {
        assert_eq!(MAX_CODEPOINT, 0x10FFFF);
    }

    #[test]
    fn first_custom() {
        assert_eq!(FIRST_CUSTOM, MAX_CODEPOINT + 1);
        assert!(FIRST_CUSTOM > MAX_CODEPOINT);
    }
}
