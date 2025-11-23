use crate::{Transport, TransportError, TransportItemRequirements};
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

//TODO: recv should return a Vec<T> from all transports using recv_blocking and ignoring any errors that are NoData
//`futures::join!`
impl<T: TransportItemRequirements> Transport<T> for List<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        let mut err = vec![];
        for transport in self.transports.iter() {
            if let Err(e) = transport.send_blocking(data.clone()) {
                err.push(e);
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
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        {
            let mut index = self.index.lock().map_err(|e| {
                TransportError::Transport(format!("Error acquiring index lock {}", e.to_string()))
            })?;
            let transport = &self.transports[*index];
            *index = (*index + 1) % self.transports.len();
            transport
        }
        .recv_blocking()
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        let mut data_vec = Vec::new();
        for transport in self.transports.iter() {
            if let Ok(data) = transport.recv_avaliable_blocking() {
                data_vec.extend(data);
            }
        }
        Ok(data_vec)
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        for transport in self.transports.iter() {
            if let Ok(Some(data)) = transport.try_recv_blocking() {
                return Ok(Some(data));
            }
        }
        Ok(None)
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
        let mut futures = vec![];
        for transport in self.transports.iter() {
            futures.push(transport.send(data.clone()));
        }
        Box::pin(async {
            let mut err = vec![];
            for fut in futures {
                if let Err(e) = fut.await {
                    err.push(e);
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
            {
                let mut index = self.index.lock().map_err(|e| {
                    TransportError::Transport(format!(
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
            let mut data_vec = Vec::new();
            for transport in self.transports.iter() {
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
            for transport in self.transports.iter() {
                if let Ok(Some(data)) = transport.try_recv().await {
                    return Ok(Some(data));
                }
            }
            Ok(None)
        })
    }
}
