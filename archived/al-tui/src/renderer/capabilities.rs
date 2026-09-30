#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Capabilities {
    pub supports_true_color: bool,
    pub supports_256_color: bool,
    pub supports_16_color: bool,
    pub supports_mouse: bool,
    pub supports_unicode: bool,
    pub supports_resizing: bool,
    pub min_width: u16,
    pub min_height: u16,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl Capabilities {
    pub fn detect() -> Self {
        //TODO: Implement real detection logic based on environment variables, terminfo, etc
        Self {
            supports_true_color: true,
            supports_256_color: true,
            supports_16_color: true,
            supports_mouse: true,
            supports_unicode: true,
            supports_resizing: true,
            min_width: 0,
            min_height: 0,
        }
    }
}
