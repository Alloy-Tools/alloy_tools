use crate::{transports::BoundaryQueue, TransportItemRequirements};
use al_structures::{
    cancellation::{CancellationFuture, CancellationToken},
    enums::{ControlFlow, Which},
    Race,
};
use std::{future::Future, sync::Arc};

/// Any error's will cause the splice to panic, displaying the error
pub fn panic_on_error(error: &dyn std::error::Error) -> ControlFlow {
    panic!("splice error: {}", error);
}

/// Any error's will be logged but ignored by the splice
pub fn log_on_error(error: &dyn std::error::Error) -> ControlFlow {
    eprintln!("splice error, stopping: {}", error);
    ControlFlow::Break
}

/// A handle that can be used to stop a splice operation.
/// Call `stop()` to signal the splice to exit its loop.
/// Await `stopped()` to pause until `stop()` is called.
#[derive(Clone)]
pub struct SpliceHandle {
    token: CancellationToken,
}

impl Default for SpliceHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl SpliceHandle {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Signal the associated splice to stop running
    pub fn stop(&self) {
        self.token.cancel();
    }

    /// Check if a stop signal has been sent
    pub fn is_stopped(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Await until the splice is canceled
    pub fn stopped(&self) -> CancellationFuture {
        self.token.cancelled()
    }
}

/// Connects a source boundary queue to a destination boundary queue, transforming items asynchronously.
/// Runs until the returned handle's `stop()` method is called or the `on_error` causes a break.
///
///
/// # Example TODO ----------------
///
/// ```ignore
/// let handle = splice_async(source, dest, |x| x * 2, tokio::spawn, panic_on_error);
/// // ... later ...
/// handle.stop();
/// ```
pub async fn splice_async<
    T: TransportItemRequirements,
    N: TransportItemRequirements,
    F: Fn(T) -> N + Send + 'static,
    S: FnOnce(std::pin::Pin<Box<dyn Future<Output = ()> + Send + 'static>>),
    E: Fn(&dyn std::error::Error) -> ControlFlow + Send + 'static,
>(
    source: Arc<BoundaryQueue<T>>,
    dest: Arc<BoundaryQueue<N>>,
    transform: F,
    spawner: S,
    on_error: E,
) -> SpliceHandle {
    let handle = SpliceHandle::new();
    let handle_clone = handle.clone();

    spawner(Box::pin(async move {
        loop {
            // Race stop signal vs next item
            let race = Race::new(
                Box::pin(handle_clone.stopped()),
                Box::pin(source.recv()),
            );

            match race.await {
                // stop signal recived
                Which::A(_) => break,
                // item received
                Which::B(recv_result) => match recv_result {
                    Ok(item) => {
                        if let Err(e) = dest.send(transform(item)) {
                            if matches!(on_error(&e), ControlFlow::Break) {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        if matches!(on_error(&e), ControlFlow::Break) {
                            break;
                        }
                    }
                },
            }
        }
    }));

    handle
}

/// Connects a source boundary queue to a destination boundary queue, transforming items in a blocking loop.
/// Runs until the returned handle's `stop()` method is called or the `on_error` causes a break.
/// Meant to be spawned on its own thread.
///
/// # Example TODO ------------------------
///
/// ```ignore
/// let handle = splice_blocking(source, dest, |x| x * 2, std::thread::spawn, panic_on_error);
///  // ... later ...
/// handle.stop();
/// ```
pub fn splice_blocking<
    T: TransportItemRequirements,
    N: TransportItemRequirements,
    F: Fn(T) -> N + Send + 'static,
    S: FnOnce(Box<dyn FnOnce() + Send + 'static>),
    E: Fn(&dyn std::error::Error) -> ControlFlow + Send + 'static,
>(
    source: Arc<BoundaryQueue<T>>,
    dest: Arc<BoundaryQueue<N>>,
    transform: F,
    spawner: S,
    on_error: E,
) -> SpliceHandle {
    let handle = SpliceHandle::new();
    let handle_clone = handle.clone();

    spawner(Box::new(move || {
        loop {
            match source.recv_cancellable(&handle_clone.token) {
                Ok(Some(item)) => {
                    if let Err(e) = dest.send(transform(item)) {
                        if matches!(on_error(&e), ControlFlow::Break) {
                            break;
                        }
                    }
                }
                // stop signal
                Ok(None) => break,
                Err(e) => {
                    if matches!(on_error(&e), ControlFlow::Break) {
                        break;
                    }
                }
            }
        }
    }));

    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn splice_async_stop_stops_loop() {
        let source = BoundaryQueue::new();
        let dest = BoundaryQueue::new();

        let handle = splice_async(
            source.clone(),
            dest.clone(),
            |x| x,
            |f| {
                tokio::spawn(f);
            },
            panic_on_error,
        )
        .await;

        assert!(!handle.is_stopped());
        handle.stop();
        std::thread::sleep(Duration::from_millis(20));

        source.send(123).unwrap();
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(dest.try_recv().unwrap(), None);
    }

    #[test]
    fn splice_blocking_stop_stops_loop() {
        let source = BoundaryQueue::new();
        let dest = BoundaryQueue::new();

        let handle = splice_blocking(
            source.clone(),
            dest.clone(),
            |x| x,
            |f| {
                std::thread::spawn(f);
            },
            panic_on_error,
        );

        handle.stop();
        std::thread::sleep(Duration::from_millis(20));

        source.send(42).unwrap();
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(dest.try_recv().unwrap(), None);
    }
}
