use tokio::sync::Notify;

use crate::{Transport, TransportError, TransportItemRequirements};
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
    pub fn as_mut(&self) -> Result<MutexGuard<Vec<Arc<dyn Transport<T>>>>, TransportError> {
        self.transports.lock().map_err(|e| e.into())
    }

    pub fn push(&self, transport: Arc<dyn Transport<T>>) -> Result<(), TransportError> {
        self.transports.lock()?.push(transport);
        Ok(())
    }
}

impl<T: TransportItemRequirements> From<List<T>> for Arc<dyn Transport<T>> {
    fn from(list: List<T>) -> Self {
        Arc::new(list)
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
        f.debug_struct("List")
            .field("transports", &self.transports)
            .finish()
    }
}

//TODO: need to add the inner `ListAware<T>(Arc<dyn Trnasport<T>>)` to support calling the condvar and notifier, or would it just be in the `send` and `send_blocking` here?
impl<T: TransportItemRequirements> Transport<T> for List<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        let mut err = vec![];
        match self.transports.lock() {
            Ok(transports) => {
                for transport in transports.iter() {
                    if let Err(e) = transport.send_blocking(data.clone()) {
                        err.push(e);
                    }
                }
            }
            Err(e) => err.push(e.into()),
        };

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
        for transport in self.transports.lock()?.iter() {
            if let Ok(data) = transport.recv_avaliable_blocking() {
                data_vec.extend(data);
            }
        }
        Ok(data_vec)
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        for transport in self.transports.lock()?.iter() {
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
        let transports = match self.transports.lock() {
            Ok(guard) => guard.iter().cloned().collect::<Vec<_>>(),
            Err(e) => {
                let err = Err(e.into());
                return Box::pin(async move { err });
            }
        };
        Box::pin(async move {
            let mut err = vec![];
            for transport in transports.iter() {
                if let Err(e) = transport.send(data.clone()).await {
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
        let transports = match self.transports.lock() {
            Ok(guard) => guard.iter().cloned().collect::<Vec<_>>(),
            Err(e) => {
                let err = Err(e.into());
                return Box::pin(async move { err });
            }
        };
        Box::pin(async {
            for transport in transports.iter() {
                if let Ok(Some(data)) = transport.try_recv().await {
                    return Ok(data);
                }
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
