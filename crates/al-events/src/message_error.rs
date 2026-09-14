use al_structures::serde_utils::RegistryError;

#[derive(Debug)]
pub enum MessageError {
    RegistryError(RegistryError),
    TypeNotRegistered(String),
    Custom(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistryError(err) => err.fmt(f),
            Self::TypeNotRegistered(type_name) => write!(f, "Message type '{}' is not registered", type_name),
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => Some(err.as_ref()),
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

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for MessageError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for MessageError {
    fn from(msg: String) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<&str> for MessageError {
    fn from(msg: &str) -> Self {
        Self::Custom(msg.to_owned().into())
    }
}
