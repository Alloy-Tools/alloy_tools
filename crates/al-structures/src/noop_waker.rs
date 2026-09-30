//! Provides a no-op task waker and context for polling futures when wake notifications are not needed.

use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
};

#[derive(Clone)]
struct NoOpWaker;

impl Wake for NoOpWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

/// Returns a shared no-op `Waker` that does nothing when woken.
///
/// # Examples
///
/// ```
/// use al_structures::noop_waker::noop_waker;
///
/// let waker = noop_waker();
/// waker.wake(); // Does nothing
/// ```
pub fn noop_waker() -> &'static Waker {
    static WAKER: std::sync::LazyLock<Waker> =
        std::sync::LazyLock::new(|| Waker::from(Arc::new(NoOpWaker)));
    &WAKER
}

/// Creates a `Context` from the shared no-op waker.
///
/// # Examples
///
/// ```
/// use al_structures::noop_waker::noop_context;
/// use std::{pin::Pin, task::Poll, future::Future};
///
/// let mut cx = noop_context();
/// let mut fut = Box::pin(async { 42 });
///
/// // Safe to poll even though waker won't be used
/// match Pin::new(&mut fut).poll(&mut cx) {
///     Poll::Ready(val) => assert_eq!(val, 42),
///     Poll::Pending => {}
/// }
/// ```
pub fn noop_context() -> Context<'static> {
    Context::from_waker(noop_waker())
}

/// Creates a new no-op `Waker` instance that ignores wake requests.
///
/// # Examples
///
/// ```
/// use al_structures::noop_waker::new_noop_waker;
///
/// let waker1 = new_noop_waker();
/// let waker2 = new_noop_waker();
///
/// waker1.wake(); // Does nothing
/// waker2.wake(); // Does nothing
/// ```

pub fn new_noop_waker() -> Waker {
    Waker::from(Arc::new(NoOpWaker))
}
