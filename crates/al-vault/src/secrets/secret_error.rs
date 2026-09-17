use al_crypto::{CryptoError, NonceError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretError {
    SerializationError(String),
    CryptoError(CryptoError),
    LockPoisoned(String),
    InvalidLength(usize),
    NonceError(NonceError),
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(str) => write!(f, "Serialization error: {str}"),
            Self::CryptoError(err) => err.fmt(f),
            Self::LockPoisoned(str) => write!(f, "Secret lock poisoned: {str}"),
            Self::InvalidLength(len) => write!(f, "Invalid length '{len}'"),
            Self::NonceError(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SecretError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CryptoError(err) => err.source(),
            Self::NonceError(err) => err.source(),
            _ => None,
        }
    }
}

impl From<bitcode::Error> for SecretError {
    fn from(value: bitcode::Error) -> Self {
        SecretError::SerializationError(value.to_string())
    }
}

impl From<CryptoError> for SecretError {
    fn from(value: CryptoError) -> Self {
        SecretError::CryptoError(value)
    }
}

impl<T> From<std::sync::PoisonError<T>> for SecretError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        SecretError::LockPoisoned(err.to_string())
    }
}

/*impl From<Vec<u8>> for SecretError {
    fn from(value: Vec<u8>) -> Self {
        SecretError::InvalidLength(value.len())
    }
}*/

impl From<NonceError> for SecretError {
    fn from(value: NonceError) -> Self {
        SecretError::NonceError(value)
    }
}
