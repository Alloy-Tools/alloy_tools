use crate::{TransportItemRequirements, TransportRequirements};
use std::{fmt::Debug, future::Future, pin::Pin, sync::PoisonError};

pub trait Transport<T: TransportItemRequirements>: TransportRequirements {
    fn send_blocking(&self, data: T) -> Result<(), TransportError>;
    fn recv_blocking(&self) -> Result<T, TransportError>;

    //TODO: make a `ReusableFutureBox` that can replace its held future without reallocating. Making the tight loops in `Link` and `Splice` more efficient
    fn send(
        &self,
        data: T,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + Sync + '_>>;
    fn recv(&self) -> Pin<Box<dyn Future<Output = Result<T, TransportError>> + Send + Sync + '_>>;
}

#[derive(Debug, Clone)]
pub enum TransportError {
    Custom(String),
    LockPoisoned(String),
    Transport(String),
    NoSend(String),
    NoRecv(String),
}

impl<T> From<PoisonError<T>> for TransportError {
    fn from(err: PoisonError<T>) -> Self {
        TransportError::LockPoisoned(err.to_string())
    }
}
