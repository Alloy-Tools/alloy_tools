//! # Boundary Queue
//!
//! A thread-safe queue for transmitting data across thread and async task boundaries.
//!
//! [`BoundaryQueue<T>`] provides a unified interface for sending and receiving items from both
//! synchronous threads and asynchronous tasks. Internally it wraps `Arc<Mutex<VecDeque<T>>>` with
//! a condition variable for thread wakeup and a waker for async notification.
//!
//! ## Features
//!
//! - **Dual-mode consumption**: Both blocking and async receivers
//! - **Batch operations**: `send_batch()` and `recv_available()` for efficiency
//! - **Waker integration**: Async tasks get notified via Waker when data arrives
//! - **Thread synchronization**: Condition variable wakes blocked threads
//! - **Cloneable**: Multiple producers/consumers via Arc cloning
//!
//! ## Examples
//!
//! ```ignore
//! use al_transport::BoundaryQueue;
//!
//! let queue = BoundaryQueue::new();
//! let queue_clone = queue.clone();
//!
//! // Producer thread
//! std::thread::spawn(move || {
//!     for i in 0..10 {
//!         queue_clone.send(i).unwrap();
//!     }
//! });
//!
//! // Consumer - asynchronously receives
//! while let Ok(item) = queue.recv().await {
//!     println!("Received: {}", item);
//! }
//! ```
//!
//! ## Error Handling
//!
//! All public methods return `Result<T, BoundaryQueueError>`. Errors occur when:
//! - The internal mutex becomes poisoned (a thread panicked while holding it)
//! - A condition variable wait operation fails (rare, system-level issue)
//!
//! ## Thread Safety
//!
//! `BoundaryQueue<T>` is `Send + Sync` for all `T: Send + Sync`, allowing safe use across
//! thread boundaries. The internal `Arc<Mutex<...>>` ensures exclusive access to the queue.

use crate::{Transport, TransportItemRequirements};
use al_structures::cancellation::CancellationToken;
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, PoisonError},
    task::Waker,
};

/// Error type for BoundaryQueue operations
#[derive(Debug, Clone, Copy)]
pub enum BoundaryQueueError {
    /// The Mutex protecting the queue became poisoned (a thread panicked while holding it)
    PoisonedMutex,
    /// The Condvar wait operation failed
    WaitFailed,
}

impl std::fmt::Display for BoundaryQueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryQueueError::PoisonedMutex => {
                write!(f, "BoundaryQueue mutex was poisoned by a panicked thread")
            }
            BoundaryQueueError::WaitFailed => {
                write!(f, "condition variable wait failed")
            }
        }
    }
}

impl std::error::Error for BoundaryQueueError {}

pub struct BoundaryQueue<T> {
    inner: Mutex<Inner<T>>,
    condvar: Arc<Condvar>,
}

struct Inner<T> {
    queue: VecDeque<T>,
    async_waker: Option<Waker>,
}

impl<T: TransportItemRequirements> From<Arc<BoundaryQueue<T>>> for Box<dyn Transport<T>> {
    fn from(value: Arc<BoundaryQueue<T>>) -> Self {
        Box::new(BoundaryQueueTransport::new(value))
    }
}

impl<T: TransportItemRequirements> BoundaryQueue<T> {
    /// Create a new BoundaryQueue, returning it wrapped in Arc for thread-safe sharing
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                queue: VecDeque::new(),
                async_waker: None,
            }),
            condvar: Arc::new(Condvar::new()),
        })
    }

    fn push(&self, data: T) -> Result<(), BoundaryQueueError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?;
        inner.queue.push_back(data);
        if let Some(w) = inner.async_waker.take() {
            w.wake();
        }
        self.condvar.notify_one();
        Ok(())
    }

    fn push_batch(&self, data: Vec<T>) -> Result<(), BoundaryQueueError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?;
        inner.queue.extend(data);
        if let Some(w) = inner.async_waker.take() {
            w.wake();
        }
        self.condvar.notify_one();
        Ok(())
    }

    /// Send a single item into the queue
    pub fn send(&self, data: T) -> Result<(), BoundaryQueueError> {
        self.push(data)
    }

    /// Send multiple items into the queue
    pub fn send_batch(&self, data: Vec<T>) -> Result<(), BoundaryQueueError> {
        self.push_batch(data)
    }

    /// Non-blocking attempt to receive data from the queue
    pub fn try_recv(&self) -> Result<Option<T>, BoundaryQueueError> {
        Ok(self
            .inner
            .lock()
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?
            .queue
            .pop_front())
    }

    /// Drain and return all currently available items from the queue
    pub fn recv_available(&self) -> Result<Vec<T>, BoundaryQueueError> {
        Ok(self
            .inner
            .lock()
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?
            .queue
            .drain(..)
            .collect())
    }

    /// Blocking receive that checks a cancellation token.
    /// Returns `Ok(None)` if the cancellation flag was raised before an item arrived.
    pub fn recv_cancellable(
        self: &Arc<Self>,
        token: &CancellationToken,
    ) -> Result<Option<T>, BoundaryQueueError> {
        let _waiter = token.cancelled_blocking({
            let queue = self.clone();
            Arc::new(move || {
                let _guard = queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                queue.condvar.notify_all();
            })
        });
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| BoundaryQueueError::PoisonedMutex)?;

        loop {
            if token.is_cancelled() {
                return Ok(None);
            }

            if let Some(data) = inner.queue.pop_front() {
                return Ok(Some(data));
            }

            inner = self
                .condvar
                .wait(inner)
                .map_err(|_| BoundaryQueueError::PoisonedMutex)?;
        }
    }

    /// Synchronously wait to receive one item from the queue.
    /// Blocks the current thread until data is available.
    pub fn recv_blocking(&self) -> Result<T, BoundaryQueueError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| BoundaryQueueError::PoisonedMutex)?;
        loop {
            if let Some(data) = inner.queue.pop_front() {
                return Ok(data);
            }
            inner = self
                .condvar
                .wait(inner)
                .map_err(|_| BoundaryQueueError::PoisonedMutex)?;
        }
    }

    /// Asynchronously wait to receive one item from the queue
    pub async fn recv(self: &Arc<Self>) -> Result<T, BoundaryQueueError> {
        RecvFuture {
            queue: self.clone(),
        }
        .await
    }
}

struct RecvFuture<T: TransportItemRequirements> {
    queue: Arc<BoundaryQueue<T>>,
}

impl<T: TransportItemRequirements> std::future::Future for RecvFuture<T> {
    type Output = Result<T, BoundaryQueueError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut inner = self
            .queue
            .inner
            .lock()
            .map_err(|_| BoundaryQueueError::PoisonedMutex)?;
        if let Some(data) = inner.queue.pop_front() {
            return std::task::Poll::Ready(Ok(data));
        }
        // Store waker for later wake-up from handle_incoming
        inner.async_waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}

/// A simple wrapper around an `Arc<BoundaryQueue<T>>` allowing `Box<dyn Transport<T>>`
pub struct BoundaryQueueTransport<T: TransportItemRequirements> {
    inner: Arc<BoundaryQueue<T>>,
}

impl<T: TransportItemRequirements> BoundaryQueueTransport<T> {
    pub fn new(queue: Arc<BoundaryQueue<T>>) -> Self {
        Self { inner: queue }
    }
}

impl<T: TransportItemRequirements> Transport<T> for BoundaryQueueTransport<T> {
    fn handle_incoming(&mut self, data: T) {
        // TODO: handle possible error
        self.inner
            .push(data)
            .expect("BoundaryQueue mutex poisoned in handle_incoming");
    }

    fn poll_action(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<crate::Action<T>> {
        // TODO: handle possible error
        let mut inner = self
            .inner
            .inner
            .lock()
            .expect("BoundaryQueue mutex poisoned in poll_action");
        if let Some(data) = inner.queue.pop_front() {
            std::task::Poll::Ready(crate::Action::Data(data))
        } else {
            inner.async_waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }

    fn status(&self) -> String {
        let queue_len = self
            .inner
            .inner
            .lock()
            .map(|inner| inner.queue.len())
            .unwrap_or_else(|_| 0);
        format!("Boundary Queue Length: {}", queue_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_structures::{cancellation::CancellationToken, noop_waker::noop_waker};

    #[test]
    fn send_and_try_recv_single_item() {
        let queue = BoundaryQueue::new();
        queue.send(10).unwrap();
        let result = queue.try_recv().unwrap();
        assert_eq!(result, Some(10));
    }

    #[test]
    fn try_recv_empty_queue_returns_none() {
        let queue = BoundaryQueue::<u8>::new();
        let result = queue.try_recv().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn send_multiple_items() {
        let queue = BoundaryQueue::new();
        queue.send(1).unwrap();
        queue.send(2).unwrap();
        queue.send(3).unwrap();

        assert_eq!(queue.try_recv().unwrap(), Some(1));
        assert_eq!(queue.try_recv().unwrap(), Some(2));
        assert_eq!(queue.try_recv().unwrap(), Some(3));
        assert_eq!(queue.try_recv().unwrap(), None);
    }

    #[test]
    fn send_batch() {
        let queue = BoundaryQueue::new();
        let batch = vec![1, 2, 3, 4, 5];
        queue.send_batch(batch).unwrap();

        for i in 1..=5 {
            assert_eq!(queue.try_recv().unwrap(), Some(i));
        }
        assert_eq!(queue.try_recv().unwrap(), None);
    }

    #[test]
    fn recv_available() {
        let queue = BoundaryQueue::new();
        queue.send(1).unwrap();
        queue.send(2).unwrap();
        queue.send(3).unwrap();

        let items = queue.recv_available().unwrap();
        assert_eq!(items, vec![1, 2, 3]);

        let items = queue.recv_available().unwrap();
        assert_eq!(items, Vec::<u8>::new());
    }

    #[tokio::test]
    async fn recv_async() {
        let queue = BoundaryQueue::new();
        let queue_clone = queue.clone();

        tokio::spawn(async move {
            queue_clone.send(42).unwrap();
        });

        let result = queue.recv().await.unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn recv_blocking() {
        let queue = BoundaryQueue::new();
        let queue_clone = queue.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            queue_clone.send(99).unwrap();
        });

        let result = queue.recv_blocking().unwrap();
        assert_eq!(result, 99);
    }

    #[test]
    fn handle_incoming() {
        let queue = BoundaryQueue::new();
        let mut transport = BoundaryQueueTransport::new(queue.clone());

        transport.handle_incoming(123);

        let result = queue.try_recv().unwrap();
        assert_eq!(result, Some(123));
    }

    #[test]
    fn poll_action_with_data() {
        let queue = BoundaryQueue::new();
        queue.send(456).unwrap();

        let mut transport = BoundaryQueueTransport::new(queue.clone());

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);
        let result = transport.poll_action(&mut cx);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = result {
            assert_eq!(data, 456);
        } else {
            panic!("Expected Ready with data");
        }
    }

    #[test]
    fn poll_action_empty() {
        let queue = BoundaryQueue::<i32>::new();
        let mut transport = BoundaryQueueTransport::new(queue);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);
        let result = transport.poll_action(&mut cx);

        if let std::task::Poll::Pending = result {
            // Expected
        } else {
            panic!("Expected Pending");
        }
    }

    #[test]
    fn status() {
        let queue = BoundaryQueue::new();
        let transport = BoundaryQueueTransport::new(queue.clone());

        queue.send(1).unwrap();
        queue.send(2).unwrap();

        let status = transport.status();
        assert!(
            status.contains("2"),
            "Status should indicate 2 items queued"
        );
    }

    #[test]
    fn fifo_order() {
        let queue = BoundaryQueue::new();
        for i in 1..=5 {
            queue.send(i).unwrap();
        }

        for i in 1..=5 {
            let result = queue.try_recv().unwrap();
            assert_eq!(result, Some(i), "Items should be in FIFO order");
        }
    }

    #[test]
    fn recv_cancellable_returns_none_after_cancel() {
        let queue = BoundaryQueue::<i32>::new();
        let token = CancellationToken::new();
        let queue_clone = queue.clone();
        let token_clone = token.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = queue_clone.recv_cancellable(&token_clone).unwrap();
            tx.send(result).unwrap();
        });

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(token.cancel());
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
            None
        );
    }

    #[test]
    fn recv_cancellable_returns_data_when_available() {
        let queue = BoundaryQueue::new();
        let token = CancellationToken::new();

        queue.send(33).unwrap();
        assert_eq!(queue.recv_cancellable(&token).unwrap(), Some(33));
    }
}
