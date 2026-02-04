use tokio::time::error::Elapsed;

use crate::NoiseError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TcpError {
    IoError(String),
    NoiseError(NoiseError),
    Timeout(String),
    SerdeError(String),
    TransportError(al_core::TransportError),
    HandshakeIncomplete,
}

impl std::fmt::Display for TcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for TcpError {
    fn from(value: std::io::Error) -> Self {
        TcpError::IoError(value.to_string())
    }
}

impl From<NoiseError> for TcpError {
    fn from(value: NoiseError) -> Self {
        TcpError::NoiseError(value)
    }
}

impl From<Elapsed> for TcpError {
    fn from(value: Elapsed) -> Self {
        TcpError::Timeout(value.to_string())
    }
}

impl From<al_core::TransportError> for TcpError {
    fn from(value: al_core::TransportError) -> Self {
        TcpError::TransportError(value)
    }
}
