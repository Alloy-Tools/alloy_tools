use al_crypto::{CryptoError, NonceError};

#[derive(Debug, Clone)]
pub enum SecretError {
    SerializationError(String),
    CryptoError(CryptoError),
    LockPoisoned(String),
    InvalidLength(usize),
    NonceError(NonceError),
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

impl From<Vec<u8>> for SecretError {
    fn from(value: Vec<u8>) -> Self {
        SecretError::InvalidLength(value.len())
    }
}

impl From<NonceError> for SecretError {
    fn from(value: NonceError) -> Self {
        SecretError::NonceError(value)
    }
}
