use crate::{Transport, TransportItemRequirements};
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
    condvar: Condvar,
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
            condvar: Condvar::new(),
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

    /// Synchronously wait to receive one item from the queue.
    /// Blocks the current thread until data is available.
    pub fn recv_blocking(&self) -> Result<T, BoundaryQueueError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?;
        loop {
            if let Some(data) = inner.queue.pop_front() {
                return Ok(data);
            }
            inner = self
                .condvar
                .wait(inner)
                .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?;
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
            .map_err(|_: PoisonError<_>| BoundaryQueueError::PoisonedMutex)?;
        if let Some(data) = inner.queue.pop_front() {
            return std::task::Poll::Ready(Ok(data));
        }
        // Store waker for later wake-up from handle_incoming
        inner.async_waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}

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
        assert_eq!(items, vec![]);
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

        let waker = crate::noop_waker().clone();
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

        let waker = crate::noop_waker().clone();
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
}
