use crate::{transport::Transport, TransportError, TransportItemRequirements};
use std::{collections::VecDeque, sync::Mutex};

/// Queue transport to implement FIFO transport
pub struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    notifier: tokio::sync::Notify,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Queue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.queue.lock() {
            Ok(queue) => f.debug_struct("Queue").field("queue", &*queue).finish(),
            Err(e) => f
                .debug_struct("Queue")
                .field("queue", &format!("<lock poisoned>: {}", e.to_string()))
                .finish(),
        }
    }
}

impl<T: TransportItemRequirements> From<Queue<T>> for std::sync::Arc<dyn Transport<T>> {
    fn from(queue: Queue<T>) -> Self {
        std::sync::Arc::new(queue)
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue::<T> {
            queue: Mutex::new(VecDeque::<T>::new()),
            notifier: tokio::sync::Notify::new(),
        }
    }
}

/// Impl transport for queue in FIFO order, handling the inner mutex for synchronization.
/// `std::Mutex` is used rather than `tokio::Mutex` for lower overhead with the restriction of not holding locks across an `await`.
impl<T: TransportItemRequirements> Transport<T> for Queue<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => Ok(guard.push_back(data)),
            Err(e) => Err(e.into()),
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => guard
                .pop_front()
                .ok_or_else(|| TransportError::Transport("No data in Queue".to_string())),
            Err(e) => Err(e.into()),
        }
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.queue.lock().unwrap().push_back(data);
        self.notifier.notify_one();
        Box::pin(async { Ok(()) })
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            loop {
                if let Some(data) = self.queue.lock().unwrap().pop_front() {
                    return Ok(data);
                }

                self.notifier.notified().await;
            }
        })
    }
}
