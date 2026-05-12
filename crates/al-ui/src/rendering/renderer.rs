use crate::{CoordType, FlushMode, GlyphId, Point, Rect, Style};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RendererError {
    RenderError(String),
}

impl From<String> for RendererError {
    fn from(s: String) -> Self {
        RendererError::RenderError(s)
    }
}

/// Backend-agnostic renderer trait
/// Implementors provide rendering to their specific backend (terminal, GUI, etc.)
/// Each backend handles glyphs internally but must support unicode ids directly.
pub trait Renderer: Send + Sync {
    type Coord: CoordType;
    /// Render a single glyph by ID at position (x, y) with given style.
    fn draw_glyph(
        &mut self,
        pos: Point<Self::Coord, 2>,
        clip: Rect<Self::Coord>,
        glyph_id: GlyphId,
        style: Style,
    ) -> Result<(), RendererError>;

    /// Render multiple glyphs, starting at position (x, y), with given style.
    fn draw_glyphs(
        &mut self,
        pos: Point<Self::Coord, 2>,
        clip: Rect<Self::Coord>,
        glyph_ids: &[GlyphId],
        style: Style,
    ) -> Result<(), RendererError>;

    /// Optimized: Render text assuming monospace font.
    /// Uses RendererMetrics for sizing (set once, not per-glyph).
    fn draw_text(
        &mut self,
        pos: Point<Self::Coord, 2>,
        text: &str,
        style: Style,
        clip: Rect<Self::Coord>,
    ) -> Result<(), RendererError>;

    /// Get glyph metrics for a specific GlyphId.
    /// Backend owns the mapping and can return rich metrics when available.
    fn get_glyph_metrics(&self, glyph_id: GlyphId) -> Option<crate::GlyphMetrics<Self::Coord>>;

    fn draw_rect(
        &mut self,
        rect: Rect<Self::Coord>,
        clip: Rect<Self::Coord>,
        color: crate::Color,
        border_color: Option<crate::Color>,
    ) -> Result<(), RendererError>;

    /// Get ui dimensions
    fn dimensions(&self) -> Result<(Self::Coord, Self::Coord), RendererError>;

    /// Flush rendered content to output; apply flush mode strategy
    fn flush(&mut self) -> Result<(), RendererError>;

    /// Set flush mode for rendering strategy
    fn set_flush_mode(&mut self, mode: FlushMode);

    /// Get current flush mode
    fn get_flush_mode(&self) -> FlushMode;

    /// Show cursor at position (x, y)
    fn show_cursor(&mut self, x: Self::Coord, y: Self::Coord) -> Result<(), RendererError>;

    /// Hide cursor
    fn hide_cursor(&mut self) -> Result<(), RendererError>;

    /// Clear entire screen
    fn clear(&mut self) -> Result<(), RendererError>;

    /// Mark entire screen as dirty
    fn mark_all_dirty(&mut self);
}
