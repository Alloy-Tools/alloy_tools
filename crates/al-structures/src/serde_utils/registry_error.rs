use crate::{
    collections::storage::utils::HandleError,
    traits::{CloneEqError, StringError},
};

//REVIEW: add `Clone, PartialEq, Eq` using a `CloneEqError`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    HandleError(HandleError),
    LockPoisoned(String),
    AlreadyRegistered(String),
    IoError(String, std::io::ErrorKind),
    ConversionFailed(Box<dyn CloneEqError>),
    InvalidId(String),
    Serialization(String),
    Deserialization(String),
    Custom(Box<dyn CloneEqError>),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleError(err) => err.fmt(f),
            Self::LockPoisoned(str) => write!(f, "Regitry lock poisoned: {str}"),
            Self::AlreadyRegistered(str) => write!(
                f,
                "A registration already exists (consider using a custom name): {str}"
            ),
            Self::IoError(str, kind) => write!(f, "Registry I/O error: {str} (Kind: {kind})"),
            Self::ConversionFailed(err) => err.fmt(f),
            Self::InvalidId(str) => write!(f, "Invalid Id: {str}"),
            Self::Serialization(str) => write!(f, "Serialization failed: {str}"),
            Self::Deserialization(str) => write!(f, "Deserialization failed: {str}"),
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => err.source(),
            Self::HandleError(err) => err.source(),
            Self::ConversionFailed(err) => err.source(),
            _ => None,
        }
    }
}

impl From<HandleError> for RegistryError {
    fn from(value: HandleError) -> Self {
        Self::HandleError(value)
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value.to_string(), value.kind())
    }
}

impl<'a, T> From<std::sync::PoisonError<std::sync::MutexGuard<'a, T>>> for RegistryError {
    fn from(value: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>) -> Self {
        Self::LockPoisoned(format!("Mutex poisoned: {value}"))
    }
}

impl From<Box<dyn CloneEqError>> for RegistryError {
    fn from(err: Box<dyn CloneEqError>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for RegistryError {
    fn from(msg: String) -> Self {
        Self::Custom(Box::new(StringError(msg)))
    }
}

impl From<&str> for RegistryError {
    fn from(msg: &str) -> Self {
        Self::Custom(Box::new(StringError(msg.to_owned())))
    }
}
