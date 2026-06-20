//! Cancellation primitives for signalling and awaiting cancellation from multiple waiters.

use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::Waker,
};

struct CancellationState {
    cancelled: AtomicBool,
    next_id: AtomicUsize,
    wakers: Mutex<HashMap<usize, Waker>>,
    blocking_wakers: Mutex<HashMap<usize, Arc<dyn Fn() + Send + Sync + 'static>>>,
}

impl CancellationState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cancelled: AtomicBool::new(false),
            next_id: AtomicUsize::new(0),
            wakers: Mutex::new(HashMap::new()),
            blocking_wakers: Mutex::new(HashMap::new()),
        })
    }
}

#[derive(Clone)]
/// A cancellation token that can be shared across threads and checked for cancellation.
pub struct CancellationToken(Arc<CancellationState>);

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cancelled: {}", self.is_cancelled())
    }
}

impl CancellationToken {
    /// Creates a new cancellation token in the non-cancelled state.
    ///
    /// # Examples
    ///
    /// ```
    /// use al_structures::cancellation::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// assert!(!token.is_cancelled());
    /// ```
    pub fn new() -> Self {
        Self(CancellationState::new())
    }

    /// Cancels the token and wakes any registered waiters.
    ///
    /// Returns `true` if the token was changed to cancelled,
    /// otherwise returns `false` if it was already cancelled.
    ///
    /// # Examples
    ///
    /// ```
    /// use al_structures::cancellation::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// assert!(token.cancel()); // First cancel returns true
    /// assert!(!token.cancel()); // Second cancel returns false
    /// ```
    pub fn cancel(&self) -> bool {
        if !self.0.cancelled.swap(true, Ordering::SeqCst) {
            let wakers = self
                .0
                .wakers
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .drain()
                .map(|(_, w)| w)
                .collect::<Vec<_>>();

            let notifiers = self
                .0
                .blocking_wakers
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .drain()
                .map(|(_, cv)| cv)
                .collect::<Vec<_>>();

            for w in wakers {
                w.wake();
            }
            for notifier in notifiers {
                notifier();
            }
            true
        } else {
            false
        }
    }

    /// Returns `true` if the token has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    /// Creates a future that completes once the token is cancelled.
    ///
    /// The returned `CancellationFuture` will automatically unregister when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use al_structures::{cancellation::CancellationToken, noop_waker::noop_context};
    /// use std::{
    ///     future::Future,
    ///     pin::Pin,
    ///     task::Poll,
    /// };
    ///
    /// let token = CancellationToken::new();
    /// let mut fut = token.cancelled();
    /// let waker = noop_waker();
    /// let mut cx = Context::from_waker(&waker);
    ///
    /// assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Pending));
    /// assert!(token.cancel());
    /// assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Ready(())));
    /// ```
    pub fn cancelled(&self) -> CancellationFuture {
        CancellationFuture::new(
            self.0.clone(),
            self.0.next_id.fetch_add(1, Ordering::Relaxed),
        )
    }

    /// Registers a blocking notifier to be called when the token is cancelled.
    ///
    /// The returned `CancellationWaiter` will automatically unregister when dropped.
    ///
    /// # Examples
    ///
    /// Condvar-based example (a common blocking use from another thread):
    ///
    /// ```
    /// use al_structures::cancellation::CancellationToken;
    /// use std::{
    ///     sync::{Arc, Mutex, Condvar},
    ///     thread,
    ///     time::Duration,
    /// };
    ///
    /// let token = CancellationToken::new();
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair_for_thread = pair.clone();
    /// let token_for_thread = token.clone();
    ///
    /// let handle = thread::spawn(move || {
    ///     let pair_for_notifier = pair_for_thread.clone();
    ///     let notifier = Arc::new(move || {
    ///         let (lock, cvar) = &*pair_for_notifier;
    ///         let mut ready = lock.lock().unwrap();
    ///         *ready = true;
    ///         cvar.notify_one();
    ///     });
    ///
    ///     // Keep the waiter alive while we block on the condvar.
    ///     let _waiter = token_for_thread.cancelled_blocking(notifier);
    ///     let (lock, cvar) = &*pair_for_notifier;
    ///     let mut ready = lock.lock().unwrap();
    ///     while !*ready {
    ///         ready = cvar.wait(ready).unwrap();
    ///     }
    /// });
    ///
    /// // Give the thread a moment to register, then cancel.
    /// thread::sleep(Duration::from_millis(10));
    /// token.cancel();
    /// handle.join().unwrap();
    /// ```
    ///
    /// Simple AtomicBool example:
    ///
    /// ```
    /// use al_structures::cancellation::CancellationToken;
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    ///
    /// let token = CancellationToken::new();
    /// let called = Arc::new(AtomicBool::new(false));
    /// let called_clone = called.clone();
    /// let notifier = Arc::new(move || { called_clone.store(true, Ordering::SeqCst); });
    /// let _waiter = token.cancelled_blocking(notifier);
    /// token.cancel();
    /// assert!(called.load(Ordering::SeqCst));
    /// ```
    pub fn cancelled_blocking<F: Fn() + Send + Sync + 'static>(
        &self,
        notifier: Arc<F>,
    ) -> CancellationWaiter {
        let id = self.0.next_id.fetch_add(1, Ordering::Relaxed);
        let mut guard = self
            .0
            .blocking_wakers
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if self.0.cancelled.load(Ordering::SeqCst) {
            notifier();
            CancellationWaiter::new(self.0.clone(), id, false)
        } else {
            guard.insert(id, notifier);
            CancellationWaiter::new(self.0.clone(), id, true)
        }
    }
}

/// A RAII guard for a blocking cancellation notifier registered with a `CancellationToken`.
///
/// Semantics and lifecycle:
/// - When returned from `CancellationToken::cancelled_blocking`, the guard represents a
///   registered notifier that will be invoked when the token is cancelled.
/// - If the token is already cancelled at registration time, the notifier is invoked
///   immediately and the guard is returned in an unregistered state (it will not attempt
///   to unregister on drop).
/// - Dropping the guard unregisters the notifier if it was registered. This prevents the
///   notifier from being called after the guard goes out of scope; keep the guard alive
///   for as long as you expect the blocking waiter to remain interested in the event.
/// - Notifier invocation happens synchronously inside `CancellationToken::cancel()` while
///   holding internal locks; the notifier should avoid long-running or blocking work.
///
/// Recommended usage:
/// - Use the guard to scope the lifetime of a blocking waiter (for example, a thread
///   that waits on a `Condvar`); the notifier should signal that waiter and the guard
///   should be held until the waiter finishes waiting.
/// - Prefer lightweight notifiers (setting an `AtomicBool` or notifying a `Condvar`).
pub struct CancellationWaiter {
    state: Arc<CancellationState>,
    id: usize,
    registered: bool,
}

impl CancellationWaiter {
    fn new(state: Arc<CancellationState>, id: usize, registered: bool) -> Self {
        Self {
            state,
            id,
            registered,
        }
    }
}

impl Drop for CancellationWaiter {
    fn drop(&mut self) {
        if self.registered {
            if let Ok(mut guard) = self.state.blocking_wakers.lock() {
                guard.remove(&self.id);
            }
        }
    }
}

/// A RAII guard future that completes when the associated `CancellationToken` is cancelled.
///
/// Semantics and lifecycle:
/// - The first time the future is polled it registers the current task's `Waker` with the
///   token. If the token is already cancelled the future returns `Ready` immediately.
/// - If the token is cancelled while the future is registered, the token will wake the
///   registered waker and the future will complete with `()` on the next poll.
/// - Dropping the future will automatically unregister the waker if it was registered,
///   preventing stale wakers from accumulating.
///
/// Notes:
/// - This future is intended as a lightweight cancellation watcher for async code. Keep
///   the future alive (i.e., held by the task) while you want to observe the cancellation.
/// - Waker registration is internal and managed for you; do not attempt to reuse the
///   same `CancellationFuture` instance across unrelated tasks.
pub struct CancellationFuture {
    state: Arc<CancellationState>,
    id: usize,
    registered: bool,
}

impl CancellationFuture {
    fn new(state: Arc<CancellationState>, id: usize) -> Self {
        Self {
            state,
            id,
            registered: false,
        }
    }
}

impl Drop for CancellationFuture {
    fn drop(&mut self) {
        if self.registered {
            if let Ok(mut guard) = self.state.wakers.lock() {
                guard.remove(&self.id);
            }
        }
    }
}

impl Future for CancellationFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();

        if this.state.cancelled.load(Ordering::SeqCst) {
            return std::task::Poll::Ready(());
        }

        // Register waker and recheck to avoid any missed wakes.
        let mut guard = this.state.wakers.lock().unwrap_or_else(|e| e.into_inner());
        if this.state.cancelled.load(Ordering::SeqCst) {
            return std::task::Poll::Ready(());
        }

        guard.insert(this.id, cx.waker().clone());
        this.registered = true;
        std::task::Poll::Pending
    }
}

#[cfg(all(test, feature = "noop_waker"))]
mod tests {
    use super::*;
    use crate::noop_waker::noop_context;
    use std::{
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::Poll,
    };

    #[test]
    fn cancellation_future() {
        let token = CancellationToken::new();
        let mut fut = token.cancelled();
        let mut cx = noop_context();

        assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Pending));
        assert!(token.cancel());
        assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Ready(())));
    }

    #[test]
    fn multiple_waiters() {
        let token = CancellationToken::new();
        let mut fut1 = token.cancelled();
        let mut fut2 = token.cancelled();
        let mut cx = noop_context();

        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx), Poll::Pending));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx), Poll::Pending));
        assert!(token.cancel());
        assert!(matches!(Pin::new(&mut fut1).poll(&mut cx), Poll::Ready(())));
        assert!(matches!(Pin::new(&mut fut2).poll(&mut cx), Poll::Ready(())));
    }

    #[test]
    fn raii_drop() {
        let token = CancellationToken::new();
        let id = {
            let mut fut = token.cancelled();
            let mut cx = noop_context();
            assert!(matches!(Pin::new(&mut fut).poll(&mut cx), Poll::Pending));
            fut.id
        };
        assert!(!token.0.wakers.lock().unwrap().contains_key(&id));
    }

    #[test]
    fn blocking_cancel() {
        let token = CancellationToken::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let notifier = Arc::new(move || {
            called_clone.store(true, Ordering::SeqCst);
        });

        let _waiter = token.cancelled_blocking(notifier);
        assert!(!called.load(Ordering::SeqCst));

        token.cancel();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn blocking_already_cancelled() {
        let token = CancellationToken::new();
        assert!(token.cancel());

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let notifier = Arc::new(move || {
            called_clone.store(true, Ordering::SeqCst);
        });

        let _waiter = token.cancelled_blocking(notifier);
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn blocking_drop() {
        let token = CancellationToken::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let notifier = Arc::new(move || {
            called_clone.store(true, Ordering::SeqCst);
        });

        let waiter = token.cancelled_blocking(notifier);
        drop(waiter);

        // Even if canceled, the dropped waiter's notifier should not be called
        token.cancel();
        assert!(!called.load(Ordering::SeqCst));
    }

    #[test]
    fn multiple_blocking_waiters() {
        let token = CancellationToken::new();
        let called1 = Arc::new(AtomicBool::new(false));
        let called2 = Arc::new(AtomicBool::new(false));
        let called1_clone = called1.clone();
        let called2_clone = called2.clone();

        let notifier1 = Arc::new(move || {
            called1_clone.store(true, Ordering::SeqCst);
        });
        let notifier2 = Arc::new(move || {
            called2_clone.store(true, Ordering::SeqCst);
        });

        let _waiter1 = token.cancelled_blocking(notifier1);
        let _waiter2 = token.cancelled_blocking(notifier2);

        token.cancel();
        assert!(called1.load(Ordering::SeqCst));
        assert!(called2.load(Ordering::SeqCst));
    }
}
