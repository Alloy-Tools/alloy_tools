use crate::{Transport, TransportError, TransportItemRequirements};
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
                .finish(),
            Err(e) => f
                .debug_struct("Publisher")
                .field(
                    "subscribers",
                    &format!("<lock poisoned>: {}", e.to_string()),
                )
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
