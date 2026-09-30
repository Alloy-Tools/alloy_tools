use std::fmt::Debug;
use al_events::MessageError;
use al_secure::noise::NoiseError;
use al_structures::traits::{CloneEqError, StringError};
use tokio::time::error::Elapsed;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TcpError {
    MessageError(MessageError),
    IoError(String, std::io::ErrorKind),
    NoiseError(NoiseError),
    Timeout(String),
    //SerdeError(String),
    DriverError(al_transport::DriverError),
    HandshakeIncomplete,
    MessageTooLarge(usize, usize),
    WireQueueClosed,
    Backpressure(al_transport::Backpressure),
    Custom(Box<dyn CloneEqError>),
}

impl std::fmt::Display for TcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageError(err) => write!(f, "{err}"),
            Self::IoError(str, kind) => write!(f, "Tcp I/O error: {str} (Kind: {kind})"),
            Self::NoiseError(err) => write!(f, "{err}"),
            Self::Timeout(str) => write!(f, "Timeout occured, time elapsed: {str}"),
            Self::DriverError(err) => write!(f, "{err}"),
            Self::HandshakeIncomplete => write!(f, "The noise handshake is incomplete."),
            Self::MessageTooLarge(len, max) => write!(f, "Message size '{len}' is larger than max '{max}'"),
            Self::WireQueueClosed => write!(f, "Wire connecting queues have closed."),
            Self::Backpressure(err) => write!(f, "{err}"),
            Self::Custom(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for TcpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MessageError(err) => err.source(),
            Self::NoiseError(err) => err.source(),
            Self::DriverError(err) => err.source(),
            Self::Custom(err) => err.source(),
            _ => None,
        }
    }
}

impl From<MessageError> for TcpError {
    fn from(value: MessageError) -> Self {
        TcpError::MessageError(value)
    }
}

impl From<std::io::Error> for TcpError {
    fn from(value: std::io::Error) -> Self {
        TcpError::IoError(value.to_string(), value.kind())
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

impl From<al_transport::DriverError> for TcpError {
    fn from(value: al_transport::DriverError) -> Self {
        TcpError::DriverError(value)
    }
}

impl From<Box<dyn CloneEqError>> for TcpError {
    fn from(err: Box<dyn CloneEqError>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for TcpError {
    fn from(msg: String) -> Self {
        Self::Custom(Box::new(StringError(msg)))
    }
}

impl From<&str> for TcpError {
    fn from(msg: &str) -> Self {
        Self::Custom(Box::new(StringError(msg.to_owned())))
    }
}
