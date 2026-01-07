use tokio::sync::Notify;

use crate::{SliceDebug, Transport, TransportError, TransportItemRequirements};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub struct List<T> {
    transports: Mutex<Vec<Arc<dyn Transport<T>>>>,
    notifier: Notify,
    condvar: Condvar,
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            transports: Mutex::new(Vec::new()),
            notifier: Notify::new(),
            condvar: Condvar::new(),
        }
    }

    /// Returns a mutex guard for the inner Vec<Arc<dyn Transport<T>>>
    pub fn as_mut(&'_ self) -> Result<MutexGuard<'_, Vec<Arc<dyn Transport<T>>>>, TransportError> {
        self.transports.lock().map_err(|e| e.into())
    }

    pub fn push(&self, transport: Arc<dyn Transport<T>>) -> Result<(), TransportError> {
        self.transports.lock()?.push(transport);
        Ok(())
    }

    /// Provides access to the inner Vec<Arc<dyn Transport<T>>> via a closure
    pub fn with<F, R>(&self, f: F) -> Result<R, TransportError>
    where
        F: FnOnce(&mut Vec<Arc<dyn Transport<T>>>) -> R,
    {
        let mut guard = self
            .transports
            .lock()
            .map_err(|e| TransportError::from(e))?;
        Ok(f(&mut *guard))
    }
}

impl<T: TransportItemRequirements> From<List<T>> for Arc<dyn Transport<T>> {
    fn from(list: List<T>) -> Self {
        Arc::new(list)
    }
}

impl<T: TransportItemRequirements, U: Transport<T>> From<Vec<Arc<U>>> for List<T> {
    fn from(transports: Vec<Arc<U>>) -> Self {
        Self {
            transports: Mutex::new(
                transports
                    .into_iter()
                    .map(|a| a as Arc<dyn Transport<T>>)
                    .collect(),
            ),
            notifier: Notify::new(),
            condvar: Condvar::new(),
        }
    }
}

impl<T: TransportItemRequirements, U: Transport<T>> From<Mutex<Vec<Arc<U>>>> for List<T> {
    fn from(transports: Mutex<Vec<Arc<U>>>) -> Self {
        Self {
            // lock, convert each Arc<U> into Arc<dyn Transport<T>>, then wrap in a new Mutex
            transports: Mutex::new(
                transports
                    .into_inner()
                    .unwrap_or_else(|m| m.into_inner())
                    .into_iter()
                    .map(|a| a as Arc<dyn Transport<T>>)
                    .collect(),
            ),
            notifier: Notify::new(),
            condvar: Condvar::new(),
        }
    }
}

impl<T> From<Vec<Arc<dyn Transport<T>>>> for List<T> {
    fn from(transports: Vec<Arc<dyn Transport<T>>>) -> Self {
        Self {
            transports: Mutex::new(transports),
            notifier: Notify::new(),
            condvar: Condvar::new(),
        }
    }
}

impl<T> From<Mutex<Vec<Arc<dyn Transport<T>>>>> for List<T> {
    fn from(transports: Mutex<Vec<Arc<dyn Transport<T>>>>) -> Self {
        Self {
            transports,
            notifier: Notify::new(),
            condvar: Condvar::new(),
        }
    }
}

impl<T> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.transports.lock() {
            Ok(guard) => f
                .debug_struct("List")
                .field("transports", &SliceDebug::new(&*guard))
                .finish(),
            Err(e) => f
                .debug_struct("List")
                .field("transports", &format!("<LockPoisoned>: {}", e.to_string()))
                .finish(),
        }
    }
}

//TODO: need to add the inner `ListAware<T>(Arc<dyn Trnasport<T>>)` to support calling the condvar and notifier, or would it just be in the `send` and `send_blocking` here?
impl<T: TransportItemRequirements> Transport<T> for List<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        let mut err = vec![];
        self.with(|transports| {
            for transport in transports.iter() {
                if let Err(e) = transport.send_blocking(data.clone()) {
                    err.push(e);
                } else {
                    // Notify any waiting receivers that new data is available
                    self.condvar.notify_one();
                    self.notifier.notify_one();
                }
            }
        })?;

        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to transports: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        let mut err = vec![];
        self.with(|transports| {
            for transport in transports.iter() {
                if let Err(e) = transport.send_batch_blocking(data.clone()) {
                    err.push(e);
                } else {
                    // Notify all waiting receivers that new data is available
                    self.condvar.notify_all();
                    self.notifier.notify_waiters();
                }
            }
        })?;

        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to transports: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        let mut transports = self.transports.lock()?;

        loop {
            for transport in transports.iter() {
                if let Ok(Some(data)) = transport.try_recv_blocking() {
                    return Ok(data);
                }
            }

            transports = self.condvar.wait(transports)?;
        }
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        let mut data_vec = Vec::new();
        self.with(|transports| {
            for transport in transports.iter() {
                if let Ok(data) = transport.recv_avaliable_blocking() {
                    data_vec.extend(data);
                }
            }
        })?;
        Ok(data_vec)
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        self.with(|transports| {
            let mut ret_val = Ok(None);
            for transport in transports.iter() {
                if let Ok(Some(data)) = transport.try_recv_blocking() {
                    ret_val = Ok(Some(data));
                    break;
                }
            }
            ret_val
        })?
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
        Box::pin(async move {
            let transports =
                self.with(|transports| transports.iter().cloned().collect::<Vec<_>>())?;

            let mut err = vec![];
            for transport in transports.iter() {
                if let Err(e) = transport.send(data.clone()).await {
                    err.push(e);
                } else {
                    // Notify any waiting receivers that new data is available
                    self.notifier.notify_one();
                    self.condvar.notify_one();
                }
            }

            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error occured when sending to transports: {:?}",
                    err
                )))
            } else {
                Ok(())
            }
        })
    }

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            let transports =
                self.with(|transports| transports.iter().cloned().collect::<Vec<_>>())?;

            let mut err = vec![];

            for transport in transports.iter() {
                if let Err(e) = transport.send_batch(data.clone()).await {
                    err.push(e);
                } else {
                    // Notify all waiting receivers that new data is available
                    self.condvar.notify_all();
                    self.notifier.notify_waiters();
                }
            }
            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error occured when sending to transports: {:?}",
                    err
                )))
            } else {
                Ok(())
            }
        })
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
            let transports =
                self.with(|transports| transports.iter().cloned().collect::<Vec<_>>())?;
            loop {
                for transport in transports.iter() {
                    if let Ok(Some(data)) = transport.try_recv().await {
                        return Ok(data);
                    }
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
            let transports =
                self.with(|transports| transports.iter().cloned().collect::<Vec<_>>())?;

            let mut data_vec = Vec::new();

            for transport in transports.iter() {
                if let Ok(data) = transport.recv_avaliable().await {
                    data_vec.extend(data);
                }
            }
            Ok(data_vec)
        })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            let transports =
                self.with(|transports| transports.iter().cloned().collect::<Vec<_>>())?;

            for transport in transports.iter() {
                if let Ok(Some(data)) = transport.try_recv().await {
                    return Ok(Some(data));
                }
            }
            Ok(None)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{List, Queue, Transport};
    use std::sync::Arc;

    #[tokio::test]
    async fn debug() {
        assert_eq!(
            format!(
                "{:?}",
                Into::<List<_>>::into(vec![Arc::new(Queue::<u8>::new())])
            ),
            "List { transports: [Queue { queue: [] }] }"
        );
    }

    #[tokio::test]
    async fn send_recv() {
        let list = List::from(vec![Arc::new(Queue::<u8>::new())]);
        list.send(1).await.unwrap();
        list.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(list.recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(list.try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(list.recv_avaliable().await.unwrap(), vec![3]);

        list.send_blocking(1).unwrap();
        list.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(list.recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(list.try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(list.recv_avaliable_blocking().unwrap(), vec![3]);
    }

    #[tokio::test]
    async fn threaded() {
        let list = Arc::new(List::from(vec![Arc::new(Queue::<u8>::new())]));
        let list_clone = list.clone();
        let handle = std::thread::spawn(move || {
            let received = list_clone.recv_blocking().unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        list.send_blocking(42).unwrap();

        handle.join().unwrap();

        let list_clone = list.clone();
        let tokio_handle = tokio::spawn(async move {
            let received = list_clone.recv().await.unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        list.send(42).await.unwrap();

        tokio_handle.await.unwrap();
    }
}
