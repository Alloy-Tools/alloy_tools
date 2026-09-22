use std::sync::Arc;

/// Take ownership of the inner value if its the last holder, else clone.
///
/// Use this at the top of `handle_incoming` in transports that need an owned `T`.
/// For fan-out targets that received the last reference, this is zero-copy;
/// for shared targets it reduces to a normal clone.
pub fn consume_or_clone<T: Clone>(arc: Arc<T>) -> T {
    Arc::try_unwrap(arc).unwrap_or_else(|shared| (*shared).clone())
}

/// Get a `&mut T`, cloning once if the `Arc` is shared.
///
/// This is `Arc::make_mut`, re-exported with a name that matches
/// `consume_or_clone` so transports read consistently.
pub fn make_mut_or_clone<T: Clone>(arc: &mut Arc<T>) -> &mut T {
    Arc::make_mut(arc)
}
