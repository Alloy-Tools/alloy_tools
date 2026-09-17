use std::fmt::{Debug, Display};

// ----- AsAny -----
pub trait AsAny: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any_box(self) -> Box<dyn std::any::Any>;
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_box(self) -> Box<dyn std::any::Any> {
        Box::new(self)
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ----- Downcast -----
pub trait Downcast {
    /// Downcast to a concrete `T`.
    fn downcast<T: 'static>(self) -> Result<T, Box<dyn std::any::Any>>;

    /// Downcast to a concrete `&T`.
    fn downcast_ref<T: 'static>(&self) -> Option<&T>;

    /// Downcast to a concrete `&mut T`.
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>;

    /// Downcast to a `Box<T>`. Returns `Err(Box<Self>)` if the type does not match.
    fn downcast_box<T: 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>>;
}

impl<F: AsAny + Sized> Downcast for F {
    fn downcast<T: 'static>(self) -> Result<T, Box<dyn std::any::Any>> {
        self.as_any_box().downcast::<T>().map(|boxed| *boxed)
    }

    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }

    fn downcast_box<T: 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.as_any().is::<T>() {
            let raw = Box::into_raw(self);
            // SAFETY: type id checked above.
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

// ----- Type name -----
pub trait TypeName {
    fn module_path() -> &'static str;
    /// Helper function to return the simple names of generic types
    fn type_with_generics() -> String {
        format!("{}::{}", Self::module_path(), tynm::type_name::<Self>())
    }
}
pub trait DynTypeName {
    fn module_path(&self) -> &'static str;
    /// Helper function to return the simple names of generic types
    fn type_with_generics(&self) -> String;
}
impl<T: TypeName> DynTypeName for T {
    fn module_path(&self) -> &'static str {
        T::module_path()
    }

    fn type_with_generics(&self) -> String {
        T::type_with_generics()
    }
}

// ----- As Bytes -----
pub trait AsBytes: Sized {
    const LEN: usize;
    fn to_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn from_bytes(buffer: &[u8]) -> Result<Self, Box<dyn std::error::Error>>;
}

// ----- Header -----
pub trait Header {
    const BUF_LEN: usize;
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;

    fn decode(buffer: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    fn decode_at(offset: &mut usize, buffer: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let target = offset
            .checked_add(Self::BUF_LEN)
            .ok_or_else(|| "header offset overflow")?;
        if target > buffer.len() {
            Err(format!(
                "Buffer length too small for header with end index '{target}.'"
            ))?;
        }
        let res = Self::decode(&buffer[*offset..target]);
        if res.is_ok() {
            *offset = target;
        }
        res
    }
}

impl<T: AsBytes> Header for T {
    const BUF_LEN: usize = T::LEN;

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.to_bytes(writer)
    }

    fn decode(buffer: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        if buffer.len() != Self::BUF_LEN {
            Err(format!(
                "Expected '{}' bytes, got '{}'",
                Self::BUF_LEN,
                buffer.len()
            ))?;
        }
        Self::from_bytes(buffer)
    }
}

// ----- CloneEqError -----
pub trait CloneEqError: Debug + Display + Send + Sync + AsAny {
    fn clone_box(&self) -> Box<dyn CloneEqError>;
    fn eq_box(&self, other: &dyn CloneEqError) -> bool;
    fn error_source(&self) -> Option<&(dyn std::error::Error + 'static)>;
}

impl<T: std::error::Error + Clone + PartialEq + Eq + Send + Sync + 'static> CloneEqError for T {
    fn clone_box(&self) -> Box<dyn CloneEqError> {
        Box::new(self.clone())
    }

    fn eq_box(&self, other: &dyn CloneEqError) -> bool {
        if let Some(o) = other.as_any().downcast_ref::<T>() {
            self == o
        } else {
            false
        }
    }

    fn error_source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(self)
    }
}

impl std::error::Error for dyn CloneEqError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error_source()
    }
}

impl Clone for Box<dyn CloneEqError> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn CloneEqError> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_box(other.as_ref())
    }
}

impl Eq for Box<dyn CloneEqError> {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringError(pub String);

impl Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
