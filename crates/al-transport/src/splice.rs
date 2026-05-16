use crate::{transports::BoundaryQueue, TransportItemRequirements};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Poll, Waker},
};

pub enum ControlFlow {
    Continue,
    Break,
}

enum Which<A, B> {
    A(A),
    B(B),
}

/// Races two boxed futures. Resolves with the winner's output.
struct Race<'a, A, B> {
    a: Option<std::pin::Pin<Box<dyn Future<Output = A> + Send + 'a>>>,
    b: Option<std::pin::Pin<Box<dyn Future<Output = B> + Send + 'a>>>,
}

impl<'a, A, B> Future for Race<'a, A, B> {
    type Output = Which<A, B>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();

        // poll  a
        if let Some(fut) = &mut this.a {
            if let Poll::Ready(val) = fut.as_mut().poll(cx) {
                return Poll::Ready(Which::A(val));
            }
        }

        // poll b
        if let Some(fut) = &mut this.b {
            if let Poll::Ready(val) = fut.as_mut().poll(cx) {
                return Poll::Ready(Which::B(val));
            }
        }

        Poll::Pending
    }
}

pub fn panic_on_error(error: &dyn std::error::Error) -> ControlFlow {
    panic!("splice error: {}", error);
}

pub fn log_on_error(error: &dyn std::error::Error) -> ControlFlow {
    eprintln!("splice error, stopping: {}", error);
    ControlFlow::Break
}

/// Creates a handle that can be used to stop a splice operation.
/// Call `stop()` to signal the splice to exit its loop.
#[derive(Clone)]
pub struct SpliceHandle {
    should_stop: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl SpliceHandle {
    /// Signal the associated splice to stop running
    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.waker.lock() {
            if let Some(w) = guard.take() {
                w.wake();
            }
        }
    }

    /// Check if a stop signal has been sent
    pub fn is_stopped(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    pub fn stopped(&self) -> impl Future<Output = ()> + '_ {
        StopFuture { handle: self }
    }

    fn new() -> Self {
        Self {
            should_stop: Arc::new(AtomicBool::new(false)),
            waker: Arc::new(Mutex::new(None)),
        }
    }
}

struct StopFuture<'a> {
    handle: &'a SpliceHandle,
}

impl Future for StopFuture<'_> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.handle.should_stop.load(Ordering::SeqCst) {
            std::task::Poll::Ready(())
        } else {
            // Register waker and recheck to avoid any missed wakes.
            if let Ok(mut guard) = self.handle.waker.lock() {
                if self.handle.should_stop.load(Ordering::SeqCst) {
                    std::task::Poll::Ready(())
                } else {
                    *guard = Some(cx.waker().clone());
                    std::task::Poll::Pending
                }
            } else {
                std::task::Poll::Ready(())
            }
        }
    }
}

/// Connects a source boundary queue to a destination boundary queue, transforming items asynchronously.
/// Runs until the returned handle's `stop()` method is called or the queues are poisoned.
///
/// Requires tokio runtime to be running.
///
/// # Panics
///
/// Panics if either the source recv or destination send fails due to a poisoned mutex.
/// This indicates a thread panicked while holding the mutex and the queue is in an unusable state.
///
/// # Example
///
/// ```ignore
/// let handle = splice_async(source, dest, |x| x * 2).await;
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
        /*while !handle_clone.is_stopped() {
            match source.recv().await {
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
            }
        }*/
        loop {
            // Race stop signal vs next item
            let race = Race {
                a: Some(Box::pin(handle_clone.stopped())),
                b: Some(Box::pin(source.recv())),
            };

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
/// Meant to be spawned on its own thread. Runs until the returned handle's `stop()` method is called
/// or the queues are poisoned.
///
/// # Panics
///
/// Panics if either the source recv_blocking or destination send fails due to a poisoned mutex.
///
/// # Example
///
/// ```ignore
/// let handle = splice_blocking(source, dest, |x| x * 2);
/// std::thread::spawn(move || {
///     handle.stop(); // Called from another thread
/// });
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
            match source.recv_cancellable(&handle_clone.should_stop) {
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
