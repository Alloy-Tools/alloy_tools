use crate::CoordType;

/// This trait allows components to query backend-specific metrics
/// such as character dimensions, DPI, and available rendering features.
/// This enables a single UI description to work across terminal and GUI backends.
pub trait RendererMetrics: Send + Sync + std::fmt::Debug {
    type Coord: crate::CoordType;
    /// Height of a single line/row in backend units
    /// Terminal: 1.0 = one character cell
    /// GUI: 1.0 = one logical pixel (DPI-aware)
    fn line_height(&self) -> Self::Coord;

    /// Width of a single character in backend units (for monospace)
    /// Terminal: 1.0 = one character cell
    /// GUI: 1.0 = width of 'M' character at current font size
    fn char_width(&self) -> Self::Coord;

    /// DPI scale factor (mainly for GUI)
    /// Terminal: typically 1.0
    /// GUI: 1.0 at 96 DPI, 2.0 at 192 DPI, etc.
    fn scale_factor(&self) -> f32 {
        1.0
    }

    /// Check if backend supports a specific renderer feature
    fn supports_feature(&self, feature: &str) -> bool {
        //TODO: handle features better (maybe splitting colors, what are `basic_shapes`?)
        matches!(
            feature,
            "colors" | "unicode" | "basic_shapes" // universally supported
        )
    }

    /// Clone this metrics provider
    fn clone_metrics(&self) -> Box<dyn RendererMetrics<Coord = Self::Coord>>;

    /// Get as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<Coord: CoordType> Clone for Box<dyn RendererMetrics<Coord = Coord>> {
    fn clone(&self) -> Self {
        self.clone_metrics()
    }
}
