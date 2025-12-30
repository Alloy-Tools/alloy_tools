use crate::{SliceDebug, Transport, TransportError, TransportItemRequirements};
use std::sync::{Arc, Mutex};

pub struct Publisher<T> {
    subscribers: Arc<Mutex<Vec<Arc<dyn Transport<T>>>>>,
}

impl<T> std::fmt::Debug for Publisher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.subscribers.lock() {
            Ok(guard) => f
                .debug_struct("Publisher")
                .field("subscribers_count", &guard.len())
                .field("subscribers", &SliceDebug::new(&*guard))
                .finish(),
            Err(e) => f
                .debug_struct("Publisher")
                .field("subscribers", &format!("<LockPoisoned>: {}", e.to_string()))
                .finish(),
        }
    }
}

impl<T: TransportItemRequirements> From<Publisher<T>> for Arc<dyn Transport<T>> {
    fn from(publisher: Publisher<T>) -> Self {
        Arc::new(publisher)
    }
}

impl<T> Publisher<T> {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&self, transport: Arc<dyn Transport<T>>) {
        if let Ok(mut guard) = self.subscribers.lock() {
            guard.push(transport);
        }
    }
}

impl<T: TransportItemRequirements> Transport<T> for Publisher<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        let mut err = vec![];
        if let Ok(guard) = self.subscribers.lock() {
            for transport in guard.iter() {
                if let Err(e) = transport.send_blocking(data.clone()) {
                    err.push(e);
                }
            }
        }
        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to subscribers: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        let mut err = vec![];
        if let Ok(guard) = self.subscribers.lock() {
            for transport in guard.iter() {
                if let Err(e) = transport.send_batch_blocking(data.clone()) {
                    err.push(e);
                }
            }
        }
        if !err.is_empty() {
            Err(TransportError::Transport(format!(
                "Error(s) occured when sending to subscribers: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        Err(TransportError::UnSupported(
            "Publisher transport does not support recv".to_string(),
        ))
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
            let transports = {
                match self.subscribers.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscribers lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let mut err = vec![];
            for transport in transports.iter() {
                if let Err(e) = transport.send(data.clone()).await {
                    err.push(e);
                }
            }

            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error(s) occured when sending to subscribers: {:?}",
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
            let transports = {
                match self.subscribers.lock() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        return Err(TransportError::Transport(format!(
                            "Error acquiring subscribers lock: {}",
                            e.to_string()
                        )))
                    }
                }
            };

            let mut err = vec![];
            for transport in transports.iter() {
                if let Err(e) = transport.send_batch(data.clone()).await {
                    err.push(e);
                }
            }

            if !err.is_empty() {
                Err(TransportError::Transport(format!(
                    "Error(s) occured when sending to subscribers: {:?}",
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
        Box::pin(async { self.recv_blocking() })
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
        Box::pin(async { self.recv_avaliable_blocking() })
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
        Box::pin(async { self.try_recv_blocking() })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Publisher, Queue, Transport};
    use std::sync::Arc;

    #[tokio::test]
    async fn debug() {
        let publisher = Publisher::<u8>::new();
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 0, subscribers: [] }"
        );
        publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 1, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }] }"
        );
        publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 3, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }] }"
        );
        publisher.subscribe(Arc::new(Publisher::<u8>::new()));
        assert_eq!(
            format!("{:?}", publisher),
            "Publisher { subscribers_count: 4, subscribers: [Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, Publisher { subscribers_count: 0, subscribers: [] }, +1 more...] }"
        );
    }

    #[tokio::test]
    async fn send_recv() {
        let publisher = Publisher::<u8>::new();
        let mut subscribers = vec![];
        for _ in 0..3 {
            let subscriber = Arc::new(Queue::<u8>::new());
            publisher.subscribe(subscriber.clone());
            subscribers.push(subscriber);
        }

        // Send `Command` asynchronously
        publisher.send(1).await.unwrap();
        publisher.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(subscribers[0].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[0].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[0].recv_avaliable().await.unwrap(), vec![3]);

        // Recv `String` asynchronously
        assert_eq!(subscribers[1].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[1].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[1].recv_avaliable().await.unwrap(), vec![3]);

        // Recv `String` asynchronously
        assert_eq!(subscribers[2].recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(subscribers[2].try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(subscribers[2].recv_avaliable().await.unwrap(), vec![3]);

        // Send `Command` synchronously
        publisher.send_blocking(1).unwrap();
        publisher.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(subscribers[0].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[0].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[0].recv_avaliable_blocking().unwrap(), vec![3]);

        // Recv `String` synchronously
        assert_eq!(subscribers[1].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[1].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[1].recv_avaliable_blocking().unwrap(), vec![3]);

        // Recv `String` synchronously
        assert_eq!(subscribers[2].recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(subscribers[2].try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(subscribers[2].recv_avaliable_blocking().unwrap(), vec![3]);
    }
}
