use crate::{BoundaryQueue, TransportItemRequirements};
use std::sync::Arc;

/// Connects a source boundary to a destination boundary, transforming items.
/// Runs forever in an async context.
///
/// # Panics
///
/// Panics if either the source recv or destination send fails due to a poisoned mutex.
/// This indicates a thread panicked while holding the mutex and the queue is in an unusable state.
pub async fn splice_async<
    T: TransportItemRequirements,
    N: TransportItemRequirements,
    F: Fn(T) -> N + Send + 'static,
>(
    source: Arc<BoundaryQueue<T>>,
    dest: Arc<BoundaryQueue<N>>,
    transform: F,
) {
    loop {
        let item = source
            .recv()
            .await
            .expect("source queue poisoned in splice_async");
        dest.send(transform(item))
            .expect("destination queue poisoned in splice_async");
    }
}

/// Connects a source boundary to a destination boundary, transforming items.
/// Blocks the calling thread forever – spawn on a separate thread.
///
/// # Panics
///
/// Panics if either the source recv_blocking or destination send fails due to a poisoned mutex.
/// This indicates a thread panicked while holding the mutex and the queue is in an unusable state.
pub fn splice_blocking<
    T: TransportItemRequirements,
    N: TransportItemRequirements,
    F: Fn(T) -> N + Send + 'static,
>(
    source: Arc<BoundaryQueue<T>>,
    dest: Arc<BoundaryQueue<N>>,
    transform: F,
) {
    loop {
        let item = source
            .recv_blocking()
            .expect("source queue poisoned in splice_blocking");
        dest.send(transform(item))
            .expect("destination queue poisoned in splice_blocking");
    }
}
