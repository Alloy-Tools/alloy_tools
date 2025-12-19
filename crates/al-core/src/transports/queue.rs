use crate::{Transport, TransportError, TransportItemRequirements};
use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

/// Queue transport to implement FIFO transport
pub struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    notifier: tokio::sync::Notify,
    condvar: Condvar,
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
            condvar: Condvar::new(),
        }
    }
}

/// Impl transport for queue in FIFO order, handling the inner mutex for synchronization.
/// `std::Mutex` is used rather than `tokio::Mutex` for lower overhead with the restriction of not holding locks across an `await`.
impl<T: TransportItemRequirements> Transport<T> for Queue<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => {
                guard.push_back(data);
                self.condvar.notify_one();
                self.notifier.notify_one();
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => {
                guard.extend(data);
                self.condvar.notify_all();
                self.notifier.notify_waiters();
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        let mut guard = self.queue.lock()?;

        while guard.is_empty() {
            guard = self.condvar.wait(guard)?;
        }

        guard.pop_front().ok_or_else(|| TransportError::NoData)
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => Ok(guard.drain(..).collect()),
            Err(e) => Err(e.into()),
        }
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => Ok(guard.pop_front()),
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
        if let Err(e) = match self.queue.lock() {
            Ok(mut guard) => Ok(guard.push_back(data)),
            Err(e) => Err(e.into()),
        } {
            return Box::pin(async { Err(e) });
        }
        self.condvar.notify_one();
        self.notifier.notify_one();
        Box::pin(async { Ok(()) })
    }

    fn send_batch(
            &self,
            data: Vec<T>,
        ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>> + Send + Sync + '_>> {
        if let Err(e) = match self.queue.lock() {
            Ok(mut guard) => Ok(guard.extend(data)),
            Err(e) => Err(e.into()),
        } {
            return Box::pin(async { Err(e) });
        }
        self.condvar.notify_all();
        self.notifier.notify_waiters();
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
                match self.queue.lock() {
                    Ok(mut queue) => {
                        if let Some(item) = queue.pop_front() {
                            return Ok(item);
                        }
                    }
                    Err(e) => return Err(e.into()),
                }

                self.notifier.notified().await;
            }
        })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            match self.queue.lock() {
                Ok(mut queue) => Ok(queue.drain(..).collect()),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            match self.queue.lock() {
                Ok(mut queue) => Ok(queue.pop_front()),
                Err(e) => Err(e.into()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Command, Queue, Transport};

    #[tokio::test]
    async fn debug() {
        assert_eq!(
            format!("{:?}", Queue::<Command>::new()),
            "Queue { queue: [] }"
        );
    }

    #[tokio::test]
    async fn send_recv() {
        let queue = Queue::<Command>::new();
        queue.send(Command::Stop).await.unwrap();
        queue.send_batch(vec![Command::Stop, Command::Stop]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(queue.recv().await.unwrap(), Command::Stop);
        // Try recv `String` asynchronously
        assert_eq!(queue.try_recv().await.unwrap().unwrap(), Command::Stop);
        // Recv avaliable `String` asynchronously
        assert_eq!(queue.recv_avaliable().await.unwrap(), vec![Command::Stop]);

        queue.send_blocking(Command::Stop).unwrap();
        queue.send_batch_blocking(vec![Command::Stop, Command::Stop]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(queue.recv_blocking().unwrap(), Command::Stop);
        // Try recv `String` synchronously
        assert_eq!(queue.try_recv_blocking().unwrap().unwrap(), Command::Stop);
        // Recv avaliable `String` synchronously
        assert_eq!(queue.recv_avaliable_blocking().unwrap(), vec![Command::Stop]);
    }
}