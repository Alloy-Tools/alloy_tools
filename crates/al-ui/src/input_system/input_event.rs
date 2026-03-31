use crate::{KeyEvent, MouseEvent, ResizeEvent};

/// All possible input events
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum InputEvent {
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Window resized
    Resize(ResizeEvent),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
}

impl PartialEq for InputEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputEvent::Key(a), InputEvent::Key(b)) => a == b,
            (InputEvent::Mouse(a), InputEvent::Mouse(b)) => a == b,
            (InputEvent::Resize(a), InputEvent::Resize(b)) => a == b,
            (InputEvent::FocusGained, InputEvent::FocusGained) => true,
            (InputEvent::FocusLost, InputEvent::FocusLost) => true,
            _ => false,
        }
    }
}