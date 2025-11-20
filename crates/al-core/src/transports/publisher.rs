use crate::{Transport, TransportItemRequirements};
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
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        let mut err = vec![];
        if let Ok(guard) = self.subscribers.lock() {
            for transport in guard.iter() {
                if let Err(e) = transport.send_blocking(data.clone()) {
                    err.push(e);
                }
            }
        }
        if !err.is_empty() {
            Err(crate::TransportError::Transport(format!(
                "Error occured when sending to subscribers: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn recv_blocking(&self) -> Result<T, crate::TransportError> {
        Err(crate::TransportError::Transport(
            "Publisher transport does not support recv".to_string(),
        ))
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        let subscribers = self.subscribers.clone();
        Box::pin(async move {
            let transports = match subscribers.lock() {
                Ok(guard) => guard.clone(),
                Err(e) => {
                    return Err(crate::TransportError::Transport(format!(
                        "Error acquiring subscribers lock: {}",
                        e.to_string()
                    )))
                }
            };

            let mut err = vec![];
            for transport in transports.iter() {
                if let Err(e) = transport.send(data.clone()).await {
                    err.push(e);
                }
            }

            if !err.is_empty() {
                Err(crate::TransportError::Transport(format!(
                    "Error occured when sending to subscribers: {:?}",
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
            dyn std::prelude::rust_2024::Future<Output = Result<T, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            Err(crate::TransportError::Transport(
                "Publisher transport does not support recv".to_string(),
            ))
        })
    }
}
