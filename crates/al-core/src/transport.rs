use crate::{TransportItemRequirements, TransportRequirements};
use std::{fmt::Debug, future::Future, pin::Pin, sync::PoisonError};

pub trait Transport<T: TransportItemRequirements>: TransportRequirements {
    /// Send the data synchronously
    fn send_blocking(&self, data: T) -> Result<(), TransportError>;
    /// Wait to receive data synchronously
    fn recv_blocking(&self) -> Result<T, TransportError>;
    /// Receive all currently avaliable data synchronously
    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError>;
    /// Try to receive data synchronously if any is currently avaliable
    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError>;

    //TODO: make a `ReusableFutureBox` that can replace its held future without reallocating. Making the tight loops in `Link` and `Splice` more efficient
    /// Send the data asynchronously
    fn send(
        &self,
        data: T,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + Sync + '_>>;
    /// Wait to receive data asynchronously
    fn recv(&self) -> Pin<Box<dyn Future<Output = Result<T, TransportError>> + Send + Sync + '_>>;
    /// Receive all currently avaliable data asynchronously
    fn recv_avaliable(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<T>, TransportError>> + Send + Sync + '_>>;
    /// Try to receive data asynchronously if any is currently avaliable
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
