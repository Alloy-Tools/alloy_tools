use crate::InputEvent;

/// Backend-agnostic input device trait
/// Implementors provide event capture from their specific backend
pub trait InputDevice: Send + Sync {
    type Result<T>;
    /// Receive next input event (blocks until event available or timeout)
    fn next_event(&mut self) -> Self::Result<Option<InputEvent>>;

    /// Poll for event with timeout in milliseconds
    /// Returns None if timeout expires, Some(event) otherwise
    fn poll_event(&mut self, timeout_ms: u64) -> Self::Result<Option<InputEvent>>;

    /// Enable mouse event capture if supported
    fn enable_mouse(&mut self) -> Self::Result<()>;

    /// Disable mouse event capture
    fn disable_mouse(&mut self) -> Self::Result<()>;
}