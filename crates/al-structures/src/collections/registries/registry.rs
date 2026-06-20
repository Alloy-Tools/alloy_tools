use std::{borrow::Borrow, hash::Hash, marker::PhantomData};

// ----- Error type -----
/// Errors returned by registry operations.
///
/// Variants cover lock poisoning, initialization failures, and custom errors converted into a boxed `Error`.
#[derive(Debug)]
pub enum RegistryError {
    LockPoisoned(String),
    InitializationFailed(String),
    Custom(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockPoisoned(msg) => write!(f, "Registry lock poisoned: {msg}"),
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {msg}"),
            Self::Custom(err) => write!(f, "Custom registry error: {err}"),
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Custom(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for RegistryError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Custom(err)
    }
}

impl From<String> for RegistryError {
    fn from(msg: String) -> Self {
        Self::Custom(msg.into())
    }
}

impl From<&str> for RegistryError {
    fn from(msg: &str) -> Self {
        Self::Custom(msg.to_owned().into())
    }
}

// ----- Read operations -----
/// Read-only registry operations.
///
/// Implementors provide thread-safe accessors and snapshot methods that return
/// copies of the underlying data. Methods return `RegistryError` to surface
/// lock poison and other failures to callers.
pub trait RegistryRead<K, V> {
    // ----- Accessors -----
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    fn contains_key<Q>(&self, key: &Q) -> Result<bool, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    // ----- snapshot methods -----
    /// Returns the number of entries in the registry.
    fn len(&self) -> Result<usize, RegistryError>;
    fn is_empty(&self) -> Result<bool, RegistryError> {
        self.len().map(|n| n == 0)
    }
    /// Returns a snapshot of all key‑value pairs.
    fn entries(&self) -> Result<Vec<(K, V)>, RegistryError>
    where
        K: Clone;
    /// Returns a snapshot of all keys.
    fn keys(&self) -> Result<Vec<K>, RegistryError>
    where
        K: Clone;
    /// Returns a snapshot of all values.
    fn values(&self) -> Result<Vec<V>, RegistryError>;
}

/// Convenience extension methods for `RegistryRead`.
///
/// These methods call the fallible versions on `RegistryRead` and panic on
/// error; they are intended for situations where the user prefers an
/// infallible API and is willing to panic on internal failures.
pub trait RegistryReadExt<K, V>: RegistryRead<K, V> {
    /// Infallible `get` - panics on error.
    fn get_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).expect("Registry `get` failed")
    }

    /// Infallible `contains_key` - panics on error.
    fn contains_key_unwrap<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.contains_key(key)
            .expect("Registry `contains_key` failed")
    }

    /// Infallible `len` - panics on error.
    fn len_unwrap(&self) -> usize {
        self.len().expect("Registry `len` failed")
    }

    /// Infallible `is_empty` - panics on error.
    fn is_empty_unwrap(&self) -> bool {
        self.is_empty().expect("Registry `is_empty` failed")
    }

    /// Infallible snapshot of entries - panics on error.
    fn entries_unwrap(&self) -> Vec<(K, V)>
    where
        K: Clone,
    {
        self.entries().expect("Registry `entries` failed")
    }

    /// Infallible snapshot of keys - panics on error.
    fn keys_unwrap(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.keys().expect("Registry `keys` failed")
    }

    /// Infallible snapshot of values - panics on error.
    fn values_unwrap(&self) -> Vec<V> {
        self.values().expect("Registry `values` failed")
    }
}

// Blanket implementation
impl<T, K, V> RegistryReadExt<K, V> for T where T: RegistryRead<K, V> {}

// ----- Write operations -----
/// Write operations for a registry.
///
/// Implementors provide thread-safe mutation operations alongside the read methods.
/// All methods return `Result<_, RegistryError>` to indicate possible lock or initialization failures.
pub trait RegistryWrite<K, V>: RegistryRead<K, V> {
    // ----- Accessors -----
    fn insert(&self, key: K, value: V) -> Result<Option<V>, RegistryError>;
    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    // ----- snapshot methods -----
    /// Remove all entries.
    fn clear(&self) -> Result<(), RegistryError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain<F>(&self, f: F) -> Result<(), RegistryError>
    where
        F: FnMut(&K, &mut V) -> bool;
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(K, V)>, RegistryError>;
    /// Insert all entries from an iterator.
    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), RegistryError>;
    /// Merge another registry into this one (overwrites existing keys).
    fn merge<R>(&self, other: &R) -> Result<(), RegistryError>
    where
        R: RegistryRead<K, V>,
        K: Clone,
    {
        self.extend(other.entries()?)
    }
}

/// Convenience extension methods for `RegistryWrite`.
///
/// These methods call the fallible versions on `RegistryWrite` and panic on
/// error; they are intended for situations where the user prefers an
/// infallible API and is willing to panic on internal failures.
pub trait RegistryWriteExt<K, V>: RegistryWrite<K, V> {
    /// Infallible `insert` - panics on error.
    fn insert_unwrap(&self, key: K, value: V) -> Option<V> {
        self.insert(key, value).expect("Registry `insert` failed")
    }

    /// Infallible `remove` - panics on error.
    fn remove_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.remove(key).expect("Registry `remove` failed")
    }

    /// Infallible `clear` - panics on error.
    fn clear_unwrap(&self) {
        self.clear().expect("Registry `clear` failed")
    }

    /// Infallible `retain` - panics on error.
    fn retain_unwrap<F>(&self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.retain(f).expect("Registry `retain` failed")
    }

    /// Infallible `drain` - panics on error.
    fn drain_unwrap(&self) -> Vec<(K, V)> {
        self.drain().expect("Registry `drain` failed")
    }

    /// Infallible `extend` - panics on error.
    fn extend_unwrap<I: IntoIterator<Item = (K, V)>>(&self, iter: I) {
        self.extend(iter).expect("Registry `extend` failed")
    }

    /// Infallible `merge` - panics on error.
    fn merge_unwrap<R>(&self, other: &R)
    where
        R: RegistryRead<K, V>,
        K: Clone,
    {
        self.merge(other).expect("Registry `merge` failed")
    }
}

// Blanket implementation
impl<T, K, V> RegistryWriteExt<K, V> for T where T: RegistryWrite<K, V> {}

// ----- Read-Write registry -----
/// A complete read-write registry interface.
///
/// This trait combines `RegistryRead` and `RegistryWrite` and adds constructors
/// and lazy-initialisation helpers. Implementations must be thread-safe and
/// typically provide `Clone` and `Default` where appropriate.
pub trait Registry<K, V>: RegistryWrite<K, V> {
    // ----- Constructors -----
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the value for `key` if it exists; otherwise call `f`,
    /// insert the result, and return a clone.
    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> V;

    /// Fallible version of `get_or_insert_with`. The closure returns a `Result<V, Err>`.
    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> Result<V, RegistryError>;
}

// ----- Bulk operations -----

/// Provides bulk read access to the underlying container `C`.
///
/// This is useful when callers need direct, read-only access to the
/// underlying container type (for example, to perform efficient lookups or
/// to hand the container to another API). Implementations should take care
/// to avoid exposing internal mutable state.
///
/// # Examples
///
/// ```
/// use al_structures::collections::registries::{RegistryBulkRead, RwLockRegistry};
///
/// let registry = RwLockRegistry::from_iter([
///     ("x".to_string(), 1),
///     ("y".to_string(), 2),
/// ]);
///
/// // Use with_read for `Result`
/// let value = registry.with_read(|map| map.get("x").cloned()).unwrap();
/// assert_eq!(value, Some(1));
///
/// // Use with_read_unwrap to panic on error
/// let value = registry.with_read_unwrap(|map| map.get("y").cloned());
/// assert_eq!(value, Some(2));
/// ```
pub trait RegistryBulkRead<K, V, C> {
    /// Execute a closure with a reference to the internal container.
    fn with_read<F, R>(&self, f: F) -> Result<R, RegistryError>
    where
        F: FnOnce(&C) -> R;

    /// Panics on error. Execute a closure with a reference to the internal container.
    fn with_read_unwrap<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&C) -> R,
    {
        self.with_read(f).unwrap()
    }
}

/// Provides bulk write access to the underlying container `C`.
///
/// This complements `RegistryBulkRead` by allowing callers to perform a
/// mutation of the internal container in a single operation. Implementations
/// are responsible for running the closure with appropriate synchronization
/// and returning any error as `RegistryError`.
///
/// # Examples
///
/// ```
/// use al_structures::collections::registries::{RegistryBulkRead, RegistryBulkWrite, RwLockRegistry};
///
/// let registry = RwLockRegistry::new();
///
/// // Use with_write for `Result`
/// registry.with_write(|map| {
///     map.insert("x".to_string(), 1);
/// }).unwrap();
///
/// // Use with_write_unwrap to panic on error
/// registry.with_write_unwrap(|map| {
///     map.insert("y".to_string(), 2);
/// });
///
/// let value = registry.with_read_unwrap(|map| map.get("x").cloned());
/// assert_eq!(value, Some(1));
///
/// let value = registry.with_read_unwrap(|map| map.get("y").cloned());
/// assert_eq!(value, Some(2));
/// ```
pub trait RegistryBulkWrite<K, V, C>: RegistryBulkRead<K, V, C> {
    /// Execute a closure with a mutable reference to the internal container.
    fn with_write<F, R>(&self, f: F) -> Result<R, RegistryError>
    where
        F: FnOnce(&mut C) -> R;

    /// Panics on error. Execute a closure with a mutable reference to the internal container.
    fn with_write_unwrap<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut C) -> R,
    {
        self.with_write(f).unwrap()
    }
}

// ----- Object-safe wrappers -----

/// Object‑safe registry trait for use with `dyn`.
///
/// This mirrors `Registry<K,V>` but erases generic closures/iterators so the
/// registry can be used behind trait objects (for example, storing different
/// registry implementations in a homogeneous collection).
///
/// # Examples
///
/// ```
/// use al_structures::collections::registries::{DynRegistry, DynRegistryBox, RwLockRegistry};
///
/// let boxed: DynRegistryBox<_, _, _> = DynRegistryBox::new(RwLockRegistry::new());
/// let dyn_registry: &dyn DynRegistry<_, _> = &boxed;
///
/// dyn_registry.insert("x".to_string(), 1).unwrap();
/// assert_eq!(dyn_registry.get(&"x".to_string()).unwrap(), Some(1));
/// ```
pub trait DynRegistry<K, V>: Send + Sync {
    // ---- reads ----
    fn get(&self, key: &K) -> Result<Option<V>, RegistryError>;
    fn contains_key(&self, key: &K) -> Result<bool, RegistryError>;
    fn len(&self) -> Result<usize, RegistryError>;
    fn is_empty(&self) -> Result<bool, RegistryError> {
        self.len().map(|l| l == 0)
    }
    fn entries(&self) -> Result<Vec<(K, V)>, RegistryError>;
    fn keys(&self) -> Result<Vec<K>, RegistryError>;
    fn values(&self) -> Result<Vec<V>, RegistryError>;

    // ---- writes ----
    fn insert(&self, key: K, value: V) -> Result<Option<V>, RegistryError>;
    fn remove(&self, key: &K) -> Result<Option<V>, RegistryError>;
    fn clear(&self) -> Result<(), RegistryError>;
    fn drain(&self) -> Result<Vec<(K, V)>, RegistryError>;
    fn extend(&self, iter: &mut dyn Iterator<Item = (K, V)>) -> Result<(), RegistryError>;

    /// Merge another `DynRegistry` into this one.
    fn merge(&self, other: &dyn DynRegistry<K, V>) -> Result<(), RegistryError> {
        self.extend(&mut other.entries()?.into_iter())
    }

    // ---- lazy initialisation ----
    fn get_or_insert_with(&self, key: K, f: &mut dyn FnMut() -> V) -> Result<V, RegistryError>;

    fn get_or_try_insert_with(
        &self,
        key: K,
        f: &mut dyn FnMut() -> Result<V, RegistryError>,
    ) -> Result<V, RegistryError>;
}

/// Wrapper providing an object‑safe view (`DynRegistry<K,V>`) of a concrete `Registry<K,V>`.
///
/// Use this when you need to treat different registry implementations
/// uniformly (for example, storing heterogeneous registries in a single
/// collection or passing them through dynamic APIs). The box holds the
/// concrete registry and forwards `DynRegistry` methods to the underlying
/// implementation.
///
/// # Examples
///
/// ```
/// use al_structures::collections::registries::{RwLockRegistry, DynRegistryBox};
///
/// let boxed: DynRegistryBox<_, _, _> = DynRegistryBox::new(RwLockRegistry::new());
/// boxed.insert("x".to_string(), 1).unwrap();
/// assert_eq!(boxed.get(&"x".to_string()).unwrap(), Some(1));
/// ```
pub struct DynRegistryBox<K, V, R> {
    inner: R,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

impl<K, V, R> DynRegistryBox<K, V, R>
where
    K: Eq + Hash + Clone + 'static,
    V: Clone + 'static,
    R: Registry<K, V> + Send + Sync + 'static,
{
    pub fn new(registry: R) -> Self {
        Self {
            inner: registry,
            _key: PhantomData,
            _value: PhantomData,
        }
    }

    /// Consume the box and return the underlying concrete registry.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<K, V, R> DynRegistry<K, V> for DynRegistryBox<K, V, R>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    R: Registry<K, V> + Send + Sync + 'static,
{
    fn get(&self, key: &K) -> Result<Option<V>, RegistryError> {
        self.inner.get(key)
    }

    fn contains_key(&self, key: &K) -> Result<bool, RegistryError> {
        self.inner.contains_key(key)
    }

    fn len(&self) -> Result<usize, RegistryError> {
        self.inner.len()
    }

    fn entries(&self) -> Result<Vec<(K, V)>, RegistryError> {
        self.inner.entries()
    }

    fn keys(&self) -> Result<Vec<K>, RegistryError> {
        self.inner.keys()
    }

    fn values(&self) -> Result<Vec<V>, RegistryError> {
        self.inner.values()
    }

    fn insert(&self, key: K, value: V) -> Result<Option<V>, RegistryError> {
        self.inner.insert(key, value)
    }

    fn remove(&self, key: &K) -> Result<Option<V>, RegistryError> {
        self.inner.remove(key)
    }

    fn clear(&self) -> Result<(), RegistryError> {
        self.inner.clear()
    }

    fn drain(&self) -> Result<Vec<(K, V)>, RegistryError> {
        self.inner.drain()
    }

    fn extend(&self, iter: &mut dyn Iterator<Item = (K, V)>) -> Result<(), RegistryError> {
        self.inner.extend(iter)
    }

    fn get_or_insert_with(&self, key: K, f: &mut dyn FnMut() -> V) -> Result<V, RegistryError> {
        self.inner.get_or_insert_with(key, || f())
    }

    fn get_or_try_insert_with(
        &self,
        key: K,
        f: &mut dyn FnMut() -> Result<V, RegistryError>,
    ) -> Result<V, RegistryError> {
        self.inner.get_or_try_insert_with(key, || f())
    }
}
