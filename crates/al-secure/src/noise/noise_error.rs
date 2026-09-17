use crate::noise::cipher_state::CipherStateReturn;
use al_crypto::{CryptoError, NonceError};
use al_vault::SecretError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NoiseError {
    CipherState(CipherStateReturn),
    CryptoError(CryptoError),
    NonceError(NonceError),
    SecretError(SecretError),
    HandshakeComplete,
    RemoteEphemeralExists,
    RemoteStaticExists,
    RemoteEphemeralMissing,
    RemoteStaticMissing,
    LocalEphemeralMissing,
    LocalStaticMissing,
    BothKeysMissing,
    InvalidKeyLength(usize, String),
}

impl std::fmt::Display for NoiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CipherState(err) => err.fmt(f),
            Self::CryptoError(err) => err.fmt(f),
            Self::NonceError(err) => err.fmt(f),
            Self::SecretError(err) => err.fmt(f),
            Self::HandshakeComplete => write!(f, "Noise handshake complete."),
            Self::RemoteEphemeralExists => write!(f, "A remote ephemeral key already exists."),
            Self::RemoteStaticExists => write!(f, "A remote static key already exists."),
            Self::RemoteEphemeralMissing => write!(f, "The remote ephemeral key is missing."),
            Self::RemoteStaticMissing => write!(f, "The remote static key is missing."),
            Self::LocalEphemeralMissing => write!(f, "The local ephemeral key is missing."),
            Self::LocalStaticMissing => write!(f, "The local static key is missing."),
            Self::BothKeysMissing => write!(f, "Both the local and remote keys are missing."),
            Self::InvalidKeyLength(len, str) => write!(f, "Invalid key length '{len}', expected '{}': {str}", super::DHLEN),
        }
    }
}

impl std::error::Error for NoiseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CipherState(err) => err.source(),
            Self::CryptoError(err) => err.source(),
            Self::NonceError(err) => err.source(),
            Self::SecretError(err) => err.source(),
            _ => None,
        }
    }
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
