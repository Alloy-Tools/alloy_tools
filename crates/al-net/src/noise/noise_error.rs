use crate::CipherStateReturn;
use al_crypto::{CryptoError, NonceError};
use al_vault::SecretError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NoiseError {
    CipherState(CipherStateReturn),
    CryptoError(CryptoError),
    NonceError(NonceError),
    SecretError(SecretError),
    HandshakeComplete,
    RemoteStaticMissing,
    LocalStaticMissing,
}

impl From<CipherStateReturn> for NoiseError {
    fn from(value: CipherStateReturn) -> Self {
        NoiseError::CipherState(value)
    }
}

impl From<CryptoError> for NoiseError {
    fn from(value: CryptoError) -> Self {
        NoiseError::CryptoError(value)
    }
}

impl From<NonceError> for NoiseError {
    fn from(value: NonceError) -> Self {
        NoiseError::NonceError(value)
    }
}

impl From<SecretError> for NoiseError {
    fn from(value: SecretError) -> Self {
        NoiseError::SecretError(value)
    }
}
