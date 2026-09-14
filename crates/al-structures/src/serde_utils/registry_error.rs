use crate::collections::storage::utils::HandleError;

#[derive(Debug)]
pub enum RegistryError {
    HandleError(HandleError),
    LockPoisoned(String),
    AlreadyRegistered(String),
    IoError(std::io::Error),
    ConversionFailed(Box<dyn std::error::Error + Send + Sync>),
    InvalidId(String),
    Serialization(String),
    Deserialization(String),
    Custom(Box<dyn std::error::Error + Send + Sync + 'static>),
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
            Self::IoError(err) => err.fmt(f),
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
            Self::Custom(err) => Some(err.as_ref()),
            Self::HandleError(err) => err.source(),
            Self::IoError(err) => err.source(),
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
        Self::IoError(value)
    }
}

impl<'a, T> From<std::sync::PoisonError<std::sync::MutexGuard<'a, T>>> for RegistryError {
    fn from(value: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>) -> Self {
        Self::LockPoisoned(format!("Mutex poisoned: {value}"))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for RegistryError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for RegistryError {
    fn from(msg: String) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<&str> for RegistryError {
    fn from(msg: &str) -> Self {
        Self::Custom(msg.to_owned().into())
    }
}