use crate::{TransportItemRequirements, TransportRequirements};
use std::{fmt::Debug, future::Future, pin::Pin, sync::PoisonError};

pub trait Transport<T: TransportItemRequirements>: TransportRequirements {
    /// Synchronously send the data
    fn send_blocking(&self, data: T) -> Result<(), TransportError>;
    /// Synchronously send the data in a batch
    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError>;
    /// Synchronously wait to receive data
    fn recv_blocking(&self) -> Result<T, TransportError>;
    /// Synchronously receive all currently avaliable data
    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError>;
    /// Synchronously try to receive data, if any is currently avaliable
    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError>;

    //TODO: make a `ReusableFutureBox` that can replace its held future without reallocating. Making the tight loops in `Link` and `Splice` more efficient
    /// Asynchronously send the data
    fn send(
        &self,
        data: T,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + Sync + '_>>;
    /// Asynchronously send the data in a batch
    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + Sync + '_>>;
    /// Asynchronously wait to receive data
    fn recv(&self) -> Pin<Box<dyn Future<Output = Result<T, TransportError>> + Send + Sync + '_>>;
    /// Asynchronously receive all currently avaliable data
    fn recv_avaliable(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<T>, TransportError>> + Send + Sync + '_>>;
    /// Asynchronously try to receive data, if any is currently avaliable
    fn try_recv(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<T>, TransportError>> + Send + Sync + '_>>;
}

#[derive(Debug, Clone)]
pub enum TransportError {
    Custom(String),
    LockPoisoned(String),
    Transport(String),
    UnSupported(String),
    NoData,
}

impl<T> From<PoisonError<T>> for TransportError {
    fn from(err: PoisonError<T>) -> Self {
        TransportError::LockPoisoned(err.to_string())
    }
}
