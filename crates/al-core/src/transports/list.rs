use crate::{Transport, TransportItemRequirements};
use std::sync::{Arc, Mutex};

pub struct List<T> {
    pub transports: Arc<Vec<Arc<dyn Transport<T>>>>,
    index: Mutex<usize>,
}

impl<T: TransportItemRequirements> From<List<T>> for Arc<dyn Transport<T>> {
    fn from(list: List<T>) -> Self {
        Arc::new(list)
    }
}

impl<T> From<Vec<Arc<dyn Transport<T>>>> for List<T> {
    fn from(transports: Vec<Arc<dyn Transport<T>>>) -> Self {
        Self {
            transports: Arc::new(transports),
            index: Mutex::new(0),
        }
    }
}

impl<T> From<Arc<Vec<Arc<dyn Transport<T>>>>> for List<T> {
    fn from(transports: Arc<Vec<Arc<dyn Transport<T>>>>) -> Self {
        Self {
            transports,
            index: Mutex::new(0),
        }
    }
}

impl<T> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("transports", &self.transports)
            .field("index", &self.index)
            .finish()
    }
}

impl<T: TransportItemRequirements> Transport<T> for List<T> {
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        let mut err = vec![];
        for transport in self.transports.iter() {
            if let Err(e) = transport.send_blocking(data.clone()) {
                err.push(e);
            }
        }
        if !err.is_empty() {
            Err(crate::TransportError::Transport(format!(
                "Error occured when sending to transports: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    fn recv_blocking(&self) -> Result<T, crate::TransportError> {
        {
            let mut index = self.index.lock().map_err(|e| {
                crate::TransportError::Transport(format!(
                    "Error acquiring index lock {}",
                    e.to_string()
                ))
            })?;
            let transport = &self.transports[*index];
            *index = (*index + 1) % self.transports.len();
            transport
        }
        .recv_blocking()
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
        let mut futures = vec![];
        for transport in self.transports.iter() {
            futures.push(transport.send(data.clone()));
        }
        Box::pin(async move {
            let mut err = vec![];
            for fut in futures {
                if let Err(e) = fut.await {
                    err.push(e);
                }
            }
            if !err.is_empty() {
                Err(crate::TransportError::Transport(format!(
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
            dyn std::prelude::rust_2024::Future<Output = Result<T, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            {
                let mut index = self.index.lock().map_err(|e| {
                    crate::TransportError::Transport(format!(
                        "Error acquiring index lock {}",
                        e.to_string()
                    ))
                })?;
                let transport = &self.transports[*index];
                *index = (*index + 1) % self.transports.len();
                transport
            }
            .recv()
            .await
        })
    }
}
