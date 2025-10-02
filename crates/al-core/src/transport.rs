use crate::{TransportItemRequirements, TransportRequirements};
use std::{fmt::Debug, sync::PoisonError};

pub trait Transport<T: TransportItemRequirements>: TransportRequirements {
    fn send(&self, data: T) -> Result<(), TransportError>;
    fn recv(&self) -> Result<T, TransportError>;
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
