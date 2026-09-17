use al_structures::{
    serde_utils::RegistryError,
    traits::{CloneEqError, StringError},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageError {
    RegistryError(RegistryError),
    TypeNotRegistered(String),
    Custom(Box<dyn CloneEqError>),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistryError(err) => err.fmt(f),
            Self::TypeNotRegistered(type_name) => {
                write!(f, "Message type '{}' is not registered", type_name)
            }
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => err.source(),
            Self::RegistryError(err) => err.source(),
            _ => None,
        }
    }
}

impl From<RegistryError> for MessageError {
    fn from(value: RegistryError) -> Self {
        MessageError::RegistryError(value)
    }
}

impl From<Box<dyn CloneEqError>> for MessageError {
    fn from(err: Box<dyn CloneEqError>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for MessageError {
    fn from(msg: String) -> Self {
        Self::Custom(Box::new(StringError(msg)))
    }
}

impl From<&str> for MessageError {
    fn from(msg: &str) -> Self {
        Self::Custom(Box::new(StringError(msg.to_owned())))
    }
}
