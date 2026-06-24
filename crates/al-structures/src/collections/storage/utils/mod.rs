//! Storage trait definitions.
//!
//! - `KeyedHandle` / `KeyedStorage` are for map-like, key/value backends.
//! - `OrderedHandle` / `OrderedStorage` are for order-sensitive keyed backends.
//! - `IndexedHandle` / `IndexedStorage` are for array-like, index-keyed backends.
pub mod indexed;
pub mod keyed;
pub mod ordered;

// ----- Error types -----
/// Errors returned by storage operations.
///
/// Includes a `Custom` variant for any boxed `Error`
#[derive(Debug)]
pub enum StorageError {
    OutOfBounds(usize, usize),
    MissingValue(String),
    Custom(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds(index, len) => {
                write!(f, "Index '{index}' is out of bounds for length '{len}'")
            }
            Self::MissingValue(msg) => write!(f, "{msg}"),
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for StorageError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for StorageError {
    fn from(msg: String) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<&str> for StorageError {
    fn from(msg: &str) -> Self {
        Self::Custom(msg.to_owned().into())
    }
}

/// Errors returned by storage handle operations.
///
/// Includes a `Custom` variant for any boxed `Error`.
#[derive(Debug)]
pub enum HandleError {
    Storage(StorageError),
    LockPoisoned(String),
    InitializationFailed(String),
    ConversionFailed(Box<dyn std::error::Error + Send + Sync>),
    Custom(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockPoisoned(msg) => write!(f, "Handle lock poisoned: {msg}"),
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {msg}"),
            Self::Storage(err) => err.fmt(f),
            Self::ConversionFailed(err) => err.fmt(f),
            Self::Custom(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for HandleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => Some(err.as_ref()),
            Self::Storage(err) => err.source(),
            Self::ConversionFailed(err) => err.source(),
            _ => None,
        }
    }
}

impl From<StorageError> for HandleError {
    fn from(value: StorageError) -> Self {
        HandleError::Storage(value)
    }
}

impl<'a, T> From<std::sync::PoisonError<std::sync::MutexGuard<'a, T>>> for HandleError {
    fn from(value: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>) -> Self {
        HandleError::LockPoisoned(format!("Mutex poisoned: {value}"))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for HandleError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for HandleError {
    fn from(msg: String) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<&str> for HandleError {
    fn from(msg: &str) -> Self {
        Self::Custom(msg.to_owned().into())
    }
}

// ----- Bulk operations -----

/// Provides bulk read access to the underlying storage `S`.
///
/// This is useful when callers need direct, read-only access to the
/// underlying storage type (for example, to perform efficient lookups or
/// to hand the storage to another API). Implementations should take care
/// to avoid exposing internal mutable state.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use al_structures::collections::storage::{RwLockStorage, utils::{keyed::KeyedHandle, HandleBulkRead}};
///
/// let storage = RwLockStorage::<HashMap<String, u8>>::from_iter([
///     ("x".to_string(), 1),
///     ("y".to_string(), 2),
/// ]);
///
/// // Use with_read for `Result`
/// let value = storage.with_read(|map| map.get("x").cloned()).unwrap();
/// assert_eq!(value, Some(1));
///
/// // Use with_read_unwrap to panic on error
/// let value = storage.with_read_unwrap(|map| map.get("y").cloned());
/// assert_eq!(value, Some(2));
/// ```
pub trait HandleBulkRead<S> {
    /// Execute a closure with a reference to the internal storage.
    fn with_read<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&S) -> R;

    /// Panics on error. Execute a closure with a reference to the internal storage.
    fn with_read_unwrap<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&S) -> R,
    {
        self.with_read(f).unwrap()
    }
}

/// Provides bulk write access to the underlying storage `S`.
///
/// This complements `HandleBulkRead` by allowing callers to perform a
/// mutation of the internal storage in a single operation. Implementations
/// are responsible for running the closure with appropriate synchronization
/// and returning any error as `HandleError`.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use al_structures::collections::storage::{RwLockStorage, utils::{keyed::KeyedHandleRead, HandleBulkWrite}};
///
/// let storage = RwLockStorage::<HashMap<String, u8>>::new(HashMap::new());
///
/// // Use with_write for `Result`
/// let len = storage.with_write(|map| {
///     map.insert("x".to_string(), 1);
///     map.len()
/// }).unwrap();
///
/// assert_eq!(len, 1);
///
/// // Use with_write_unwrap to panic on error
/// let len = storage.with_write_unwrap(|map| {
///     map.insert("y".to_string(), 2);
///     let x = map.entry("x").or_insert(1);
///     *x = 3;
///     map.len()
/// });
/// assert_eq!(len, 2);
///
/// let value = storage.get("x").unwrap();
/// assert_eq!(value, Some(3));
///
/// let value = storage.get("y").unwrap();
/// assert_eq!(value, Some(2));
/// ```
pub trait HandleBulkWrite<S> {
    /// Execute a closure with a mutable reference to the internal storage.
    fn with_write<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&mut S) -> R;

    /// Panics on error. Execute a closure with a mutable reference to the internal storage.
    fn with_write_unwrap<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut S) -> R,
    {
        self.with_write(f).unwrap()
    }
}
