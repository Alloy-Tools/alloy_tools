/// Window resize event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResizeEvent {
    pub width: u16,
    pub height: u16,
}

impl ResizeEvent {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}
