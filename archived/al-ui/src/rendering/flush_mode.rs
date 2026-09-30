/// Flush strategy for renderer output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlushMode {
    /// Only render dirty regions (most efficient for small changes)
    DirtyRectOnly,
    #[default]
    /// Render entire buffer (safe fallback, less efficient)
    FullBuffer,
    /// Use double-buffer swap to eliminate tearing (higher memory, flicker-free)
    DoubleBufferSwap,
}
