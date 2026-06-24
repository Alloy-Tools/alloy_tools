use super::{HandleError, StorageError};
use std::{borrow::Borrow, hash::Hash, ops::ControlFlow};

// ----- Read operations -----
/// Read-only operations for keyed storages.
pub trait KeyedHandleRead<K, V> {
    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> R;
    /// Run `f` on every borrowed key/value pair.
    fn for_each<F: FnMut((&K, &V))>(&self, f: F) -> Result<(), HandleError>;
    /// Optional fallible variant for early exit / error propagation.
    fn try_for_each<F: FnMut((&K, &V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<ControlFlow<B>, HandleError>;
    /// Accumulate a value over every borrowed key/value pair.
    fn fold<F: FnMut(A, (&K, &V)) -> A, A>(&self, init: A, mut f: F) -> Result<A, HandleError> {
        let mut acc = Some(init);
        self.for_each(|pair| acc = Some(f(acc.take().unwrap(), pair)))?;
        Ok(acc.unwrap())
    }
    /// Collect one output for every borrowed key/value pair.
    fn map<F: FnMut((&K, &V)) -> R, R>(&self, mut f: F) -> Result<Vec<R>, HandleError> {
        let mut out = Vec::new();
        self.for_each(|pair| out.push(f(pair)))?;
        Ok(out)
    }
    /// Returns the key of the first key/value pair to satisfy `f`, if any.
    fn find_key<F>(&self, mut f: F) -> Result<Option<K>, HandleError>
    where
        K: Clone,
        F: FnMut((&K, &V)) -> bool,
    {
        if let ControlFlow::Break(key) = self.try_for_each(|(k, v)| {
            if f((k, v)) {
                ControlFlow::Break(k.clone())
            } else {
                ControlFlow::Continue(())
            }
        })? {
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }
    /// Returns the first key/value pair to satisfy `f`, if any.
    fn find<F>(&self, mut f: F) -> Result<Option<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
        F: FnMut((&K, &V)) -> bool,
    {
        if let ControlFlow::Break(pair) = self.try_for_each(|(k, v)| {
            if f((k, v)) {
                ControlFlow::Break((k.clone(), v.clone()))
            } else {
                ControlFlow::Continue(())
            }
        })? {
            Ok(Some(pair))
        } else {
            Ok(None)
        }
    }
    /// Returns `true` if the storage contains the key.
    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    /// Returns the number of entries in the storage.
    fn len(&self) -> Result<usize, HandleError>;
    /// Returns `true` if the storage is empty.
    fn is_empty(&self) -> Result<bool, HandleError> {
        self.len().map(|n| n == 0)
    }
    /// Returns a snapshot of all key‑value pairs.
    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone;
    /// Returns a snapshot of all keys.
    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone;
    /// Returns a snapshot of all values.
    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone;
}

/// Convenience extension methods for `KeyedHandleRead`.
///
/// These methods call the fallible versions on `KeyedHandleRead` and panic on error.
pub trait KeyedHandleReadExt<K, V>: KeyedHandleRead<K, V> {
    /// Infallible `get` - panics on error.
    fn get_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone,
    {
        self.get(key).expect("`get` failed")
    }

    /// Infallible `with_ref` - panics on error.
    fn with_ref_unwrap<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        self.with_ref(key, f).expect("`with_ref` failed")
    }

    /// Infallible `for_each` - panics on error.
    fn for_each_unwrap<F: FnMut((&K, &V))>(&self, f: F) {
        self.for_each(f).expect("`for_each` failed");
    }

    /// Infallible `try_for_each` - panics on error.
    fn try_for_each_unwrap<F: FnMut((&K, &V)) -> ControlFlow<B>, B>(&self, f: F) -> ControlFlow<B> {
        self.try_for_each(f).expect("`try_for_each` failed")
    }

    /// Infallible `fold` - panics on error.
    fn fold_unwrap<F: FnMut(A, (&K, &V)) -> A, A>(&self, init: A, f: F) -> A {
        self.fold(init, f).expect("`fold` failed")
    }

    /// Infallible `map` - panics on error.
    fn map_unwrap<F: FnMut((&K, &V)) -> R, R>(&self, f: F) -> Vec<R> {
        self.map(f).expect("`map` failed")
    }

    /// Infallible `find_key` - panics on error.
    fn find_key_unwrap<F>(&self, f: F) -> Option<K>
    where
        K: Clone,
        F: FnMut((&K, &V)) -> bool,
    {
        self.find_key(f).expect("`find_key` failed")
    }

    /// Infallible `find` - panics on error.
    fn find_unwrap<F>(&self, f: F) -> Option<(K, V)>
    where
        K: Clone,
        V: Clone,
        F: FnMut((&K, &V)) -> bool,
    {
        self.find(f).expect("`find` failed")
    }

    /// Infallible `contains_key` - panics on error.
    fn contains_key_unwrap<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.contains_key(key).expect("`contains_key` failed")
    }

    /// Infallible `len` - panics on error.
    fn len_unwrap(&self) -> usize {
        self.len().expect("`len` failed")
    }

    /// Infallible `is_empty` - panics on error.
    fn is_empty_unwrap(&self) -> bool {
        self.is_empty().expect("`is_empty` failed")
    }

    /// Infallible snapshot of entries - panics on error.
    fn entries_unwrap(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.entries().expect("`entries` failed")
    }

    /// Infallible snapshot of keys - panics on error.
    fn keys_unwrap(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.keys().expect("`keys` failed")
    }

    /// Infallible snapshot of values - panics on error.
    fn values_unwrap(&self) -> Vec<V>
    where
        V: Clone,
    {
        self.values().expect("`values` failed")
    }
}

// Blanket implementation
impl<T, K, V> KeyedHandleReadExt<K, V> for T where T: KeyedHandleRead<K, V> {}

// ----- Write operations -----
/// Write operations for keyed storages.
pub trait KeyedHandleWrite<K, V>: KeyedHandleRead<K, V> {
    /// Insert the passed key/value pair into the storage, returning any old value.
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError>;
    /// Get an owned value from the key, inserting the passed value if none exists.
    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone;
    /// Get an owned value from the key, attempting to insert a value if none exists using `f`.
    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>;
    /// Mutate the value for `key` via closure `f`. Returns the closure's result if present.
    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&mut V) -> R;
    /// Run `f` on every mutable key/value pair.
    fn for_each_mut<F: FnMut((&K, &mut V))>(&self, f: F) -> Result<(), HandleError>;
    /// Optional fallible mutable variant.
    fn try_for_each_mut<F: FnMut((&K, &mut V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<ControlFlow<B>, HandleError>;
    /// Accumulate a value over every key/value pair.
    fn fold_mut<F: FnMut(A, (&K, &mut V)) -> A, A>(
        &self,
        init: A,
        mut f: F,
    ) -> Result<A, HandleError> {
        let mut acc = Some(init);
        self.for_each_mut(|pair| acc = Some(f(acc.take().unwrap(), pair)))?;
        Ok(acc.unwrap())
    }
    /// Collect one output for every key/value pair.
    fn map_mut<F: FnMut((&K, &mut V)) -> R, R>(&self, mut f: F) -> Result<Vec<R>, HandleError> {
        let mut out = Vec::new();
        self.for_each_mut(|pair| out.push(f(pair)))?;
        Ok(out)
    }
    /// Remove and return any existing entry under the key.
    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;
    /// Remove all entries.
    fn clear(&self) -> Result<(), HandleError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut(&K, &mut V) -> bool;
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(K, V)>, HandleError>;
    /// Insert all entries from an iterator.
    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), HandleError>;
    /// Merge another keyed storage into this one.
    fn merge<R>(&self, other: &R) -> Result<(), HandleError>
    where
        R: KeyedHandleRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.extend(other.entries()?)
    }
}

/// Convenience extension methods for `KeyedHandleWrite`.
///
/// These methods call the fallible versions on `KeyedHandleWrite` and panics on error.
pub trait KeyedHandleWriteExt<K, V>: KeyedHandleWrite<K, V> {
    /// Infallible `insert` - panics on error.
    fn insert_unwrap(&self, key: K, value: V) -> Option<V> {
        self.insert(key, value).expect("`insert` failed")
    }

    /// Infallible `try_insert` - panics on error.
    fn try_insert_unwrap(&self, key: K, value: V) -> V
    where
        V: Clone,
    {
        self.try_insert(key, value).expect("`try_insert` failed")
    }

    /// Infallible `try_insert_with` - panics on error.
    fn try_insert_with_unwrap<F>(&self, key: K, value: F) -> V
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        self.try_insert_with(key, value)
            .expect("`try_insert_with` failed")
    }

    /// Infallible `with_mut` - panics on error.
    fn with_mut_unwrap<Q, F, R>(&mut self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        self.with_mut(key, f).expect("`with_mut` failed")
    }

    /// Infallible `for_each_mut` - panics on error.
    fn for_each_mut_unwrap<F: FnMut((&K, &mut V))>(&self, f: F) {
        self.for_each_mut(f).expect("`for_each_mut` failed")
    }

    /// Infallible `try_for_each_mut` - panics on error.
    fn try_for_each_mut_unwrap<F: FnMut((&K, &mut V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> ControlFlow<B> {
        self.try_for_each_mut(f).expect("`try_for_each_mut` failed")
    }

    /// Infallible `fold_mut` - panics on error.
    fn fold_mut_unwrap<F: FnMut(A, (&K, &mut V)) -> A, A>(&self, init: A, f: F) -> A {
        self.fold_mut(init, f).expect("`fold_mut` failed")
    }

    /// Infallible `map_mut` - panics on error.
    fn map_mut_unwrap<F: FnMut((&K, &mut V)) -> R, R>(&self, f: F) -> Vec<R> {
        self.map_mut(f).expect("`map_mut` failed")
    }

    /// Infallible `remove` - panics on error.
    fn remove_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.remove(key).expect("`remove` failed")
    }

    /// Infallible `clear` - panics on error.
    fn clear_unwrap(&self) {
        self.clear().expect("`clear` failed")
    }

    /// Infallible `retain` - panics on error.
    fn retain_unwrap<F>(&self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.retain(f).expect("`retain` failed")
    }

    /// Infallible `drain` - panics on error.
    fn drain_unwrap(&self) -> Vec<(K, V)> {
        self.drain().expect("`drain` failed")
    }

    /// Infallible `extend` - panics on error.
    fn extend_unwrap<I: IntoIterator<Item = (K, V)>>(&self, iter: I) {
        self.extend(iter).expect("`extend` failed")
    }

    /// Infallible `merge` - panics on error.
    fn merge_unwrap<R>(&self, other: &R)
    where
        R: KeyedHandleRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.merge(other).expect("`merge` failed")
    }
}

// Blanket implementation
impl<T, K, V> KeyedHandleWriteExt<K, V> for T where T: KeyedHandleWrite<K, V> {}

// ----- Read-Write union -----
/// A complete read-write keyed storage interface.
///
/// This trait combines `KeyedHandleRead` and `KeyedHandleWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait KeyedHandle<K, V>: KeyedHandleWrite<K, V> {
    type Storage;
    // ----- Constructors -----
    fn new(storage: Self::Storage) -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the value for `key` if it exists; otherwise call `f` and insert the result.
    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> V;

    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>;
}

// ----- Dyn-Compatible wrapper -----
/// Dyn-Compatible keyed handle trait.
///
/// This mirrors `KeyedHandle<K,V>` but erases generic closures/iterators so the
/// handle can be used behind trait objects (for example, storing different
/// handle implementations in a homogeneous collection).
///
/// # Examples
///
/// ```
/// use al_structures::collections::storage::{RwLockStorage, utils::keyed::KeyedDynHandle};
/// use std::collections::hash_map::HashMap;
///
/// let boxed: Box<dyn KeyedDynHandle<String, u8>> = Box::new(RwLockStorage::new(HashMap::new()));
///
/// boxed.insert("x".to_string(), 1).unwrap();
/// assert_eq!(boxed.get(&"x".to_string()).unwrap().unwrap(), 1);
/// ```
pub trait KeyedDynHandle<K, V>: Send + Sync {
    // ---- reads ----
    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Hash + Eq,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref(&self, key: &K, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        K: Hash + Eq;
    /// Run `f` on every borrowed key/value pair.
    fn for_each(&self, f: &mut dyn FnMut((&K, &V))) -> Result<(), HandleError>;
    /// Optional fallible variant for early exit / error propagation.
    fn try_for_each(
        &self,
        f: &mut dyn FnMut((&K, &V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError>;
    // Returns the key of the first key/value pair to satisfy `f`, if any.
    fn find_key(&self, f: &mut dyn FnMut((&K, &V)) -> bool) -> Result<Option<K>, HandleError>
    where
        K: Clone;
    // Returns the first key/value pair to satisfy `f`, if any.
    fn find(&self, f: &mut dyn FnMut((&K, &V)) -> bool) -> Result<Option<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone;
    /// Returns `true` if the storage contains the key.
    fn contains_key(&self, key: &K) -> Result<bool, HandleError>
    where
        K: Hash + Eq;
    /// Returns the number of entries in the storage.
    fn len(&self) -> Result<usize, HandleError>;
    /// Returns `true` if the storage is empty.
    fn is_empty(&self) -> Result<bool, HandleError> {
        self.len().map(|l| l == 0)
    }
    /// Returns a snapshot of all key‑value pairs.
    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone;
    /// Returns a snapshot of all keys.
    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone;
    /// Returns a snapshot of all values.
    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone;

    // ---- writes ----
    /// Insert the passed key/value pair into the storage, returning any old value.
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError>;
    /// Get an owned value from the key, inserting the passed value if none exists.
    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone;
    /// Get an owned value from the key, attempting to insert a value if none exists using `f`.
    fn try_insert_with(
        &self,
        key: K,
        value: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone;
    /// Mutate the value for `key` via closure `f`. Returns the closure's result if present.
    fn with_mut(&self, key: &K, f: &mut dyn FnMut(&mut V)) -> Result<bool, HandleError>
    where
        K: Hash + Eq;
    /// Run `f` on every mutable key/value pair.
    fn for_each_mut(&self, f: &mut dyn FnMut((&K, &mut V))) -> Result<(), HandleError>;
    /// Optional fallible mutable variant.
    fn try_for_each_mut(
        &self,
        f: &mut dyn FnMut((&K, &mut V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError>;
    /// Remove and return any existing entry under the key.
    fn remove(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Hash + Eq;
    /// Remove all entries.
    fn clear(&self) -> Result<(), HandleError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain(&self, f: &mut dyn FnMut(&K, &mut V) -> bool) -> Result<(), HandleError>;
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(K, V)>, HandleError>;
    /// Insert all entries from an iterator.
    fn extend(&self, iter: &mut dyn Iterator<Item = (K, V)>) -> Result<(), HandleError>;

    /// Merge another `KeyedDynHandle` into this one.
    fn merge(&self, other: &dyn KeyedDynHandle<K, V>) -> Result<(), HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.extend(&mut other.entries()?.into_iter())
    }

    // ---- lazy initialisation ----
    /// Return the value for `key` if it exists; otherwise call `f` and insert the result.
    fn get_or_insert_with(&self, key: K, f: &mut dyn FnMut() -> V) -> Result<V, HandleError>
    where
        V: Clone;
    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with(
        &self,
        key: K,
        f: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone;
}

impl<K, V, T: KeyedHandle<K, V> + Send + Sync> KeyedDynHandle<K, V> for T {
    fn get(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Hash + Eq,
        V: Clone,
    {
        self.get(key)
    }

    fn with_ref(&self, key: &K, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        K: Hash + Eq,
    {
        self.with_ref(key, f).map(|o| o.is_some())
    }

    fn for_each(&self, f: &mut dyn FnMut((&K, &V))) -> Result<(), HandleError> {
        self.for_each(f)
    }

    fn try_for_each(
        &self,
        f: &mut dyn FnMut((&K, &V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError> {
        self.try_for_each(f)
    }

    fn find_key(&self, f: &mut dyn FnMut((&K, &V)) -> bool) -> Result<Option<K>, HandleError>
    where
        K: Clone,
    {
        self.find_key(f)
    }

    fn find(&self, f: &mut dyn FnMut((&K, &V)) -> bool) -> Result<Option<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.find(f)
    }

    fn contains_key(&self, key: &K) -> Result<bool, HandleError>
    where
        K: Hash + Eq,
    {
        self.contains_key(key)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.len()
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.entries()
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.keys()
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.values()
    }

    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        self.insert(key, value)
    }

    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.try_insert(key, value)
    }

    fn try_insert_with(
        &self,
        key: K,
        value: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.try_insert_with(key, value)
    }

    fn with_mut(&self, key: &K, f: &mut dyn FnMut(&mut V)) -> Result<bool, HandleError>
    where
        K: Hash + Eq,
    {
        self.with_mut(key, f).map(|o| o.is_some())
    }

    fn for_each_mut(&self, f: &mut dyn FnMut((&K, &mut V))) -> Result<(), HandleError> {
        self.for_each_mut(f)
    }

    fn try_for_each_mut(
        &self,
        f: &mut dyn FnMut((&K, &mut V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError> {
        self.try_for_each_mut(f)
    }

    fn remove(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Hash + Eq,
    {
        self.remove(key)
    }

    fn clear(&self) -> Result<(), HandleError> {
        self.clear()
    }

    fn retain(&self, f: &mut dyn FnMut(&K, &mut V) -> bool) -> Result<(), HandleError> {
        self.retain(f)
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        self.drain()
    }

    fn extend(&self, iter: &mut dyn Iterator<Item = (K, V)>) -> Result<(), HandleError> {
        self.extend(iter)
    }

    fn get_or_insert_with(&self, key: K, f: &mut dyn FnMut() -> V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.get_or_insert_with(key, || f())
    }

    fn get_or_try_insert_with(
        &self,
        key: K,
        f: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.get_or_try_insert_with(key, || f())
    }
}

// ----- Inner Storage traits -----
/// Generic read-only behavior for keyed storage backends.
///
/// `KeyedStorageRead` is designed for containers where values are indexed by a key
/// type `K`, and lookups may be performed with a borrowed form `Q` of that key.
/// This trait is suitable for hash map storage such as `HashMap<K, V>`, and any
/// other keyed container that supports borrowed lookups.
pub trait KeyedStorageRead<K, V> {
    type Iter<'a>: Iterator<Item = (&'a K, &'a V)> + 'a
    where
        Self: 'a,
        K: 'a,
        V: 'a;
    type IterMut<'a>: Iterator<Item = (&'a K, &'a mut V)> + 'a
    where
        Self: 'a,
        K: 'a,
        V: 'a;

    /// Returns a reference to the value associated with `key`, if present.
    fn get<Q>(&self, key: &Q) -> Result<Option<&V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;

    /// Returns whether the container contains an entry for `key`.
    fn contains_key<Q>(&self, key: &Q) -> Result<bool, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;

    /// Returns the number of entries currently stored in the container.
    fn len(&self) -> Result<usize, StorageError>;

    /// Returns whether the container holds no entries.
    fn is_empty(&self) -> Result<bool, StorageError> {
        self.len().map(|l| l == 0)
    }

    /// Returns owned key/value pairs for all entries.
    fn entries(&self) -> Result<Vec<(K, V)>, StorageError>
    where
        K: Clone,
        V: Clone;

    /// Returns owned keys for all entries.
    fn keys(&self) -> Result<Vec<K>, StorageError>
    where
        K: Clone;

    /// Returns owned values for all entries.
    fn values(&self) -> Result<Vec<V>, StorageError>
    where
        V: Clone;

    /// Returns an iterator over borrowed entries.
    fn iter(&self) -> Self::Iter<'_>;
}

/// `KeyedStorageWrite` is intended for containers where keyed mutation is supported.
/// It extends `KeyedStorageRead` with insertion, replacement, and removal operations.
pub trait KeyedStorageWrite<K, V>: KeyedStorageRead<K, V> + Default {
    /// Insert a key/value pair into the container.
    /// If the key already exists, the previous value is returned.
    fn insert(&mut self, key: K, value: V) -> Result<Option<V>, StorageError>;

    /// Insert a key/value pair only when the key is absent.
    fn try_insert(&mut self, key: K, value: V) -> Result<&V, StorageError>;

    /// Insert a key/value pair only when the key is absent.
    fn try_insert_with<F>(&mut self, key: K, value: F) -> Result<&V, StorageError>
    where
        F: FnOnce() -> Result<V, StorageError>;

    /// Returns a mutable reference to the value for `key`, if present.
    fn get_mut<Q>(&mut self, key: &Q) -> Result<Option<&mut V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;

    /// Removes the entry for `key`, returning the owned value if it existed.
    fn remove<Q>(&mut self, key: &Q) -> Result<Option<V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized;

    /// Clears all entries from the container.
    fn clear(&mut self) -> Result<(), StorageError>;

    /// Retains only the entries that satisfy the predicate.
    fn retain<F>(&mut self, f: F) -> Result<(), StorageError>
    where
        F: FnMut(&K, &mut V) -> bool;

    /// Removes and returns all entries as owned pairs.
    fn drain(&mut self) -> Result<Vec<(K, V)>, StorageError>;

    /// Extends the container with owned entries from `iter`.
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) -> Result<(), StorageError>;

    /// Merge another storage into this one.
    fn merge<R>(&mut self, other: &R) -> Result<(), StorageError>
    where
        R: KeyedStorageRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.extend(other.entries()?)
    }

    /// Returns an iterator over borrowed mutable entries.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

/// A complete read-write keyed storage interface.
///
/// This trait combines `KeyedStorageRead` and `KeyedStorageWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait KeyedStorage<K, V>: KeyedStorageWrite<K, V> {
    // ----- Constructors -----
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the value for `key` if it exists; otherwise call `f`,
    /// insert the result, and return a clone.
    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> Result<&V, StorageError>;

    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with<F: FnOnce() -> Result<V, StorageError>>(
        &mut self,
        key: K,
        f: F,
    ) -> Result<&V, StorageError>;
}

// ----- HashMap storage -----
// `HashMap` storage implementation for keyed read/write behavior.
//
// It supports borrowed key lookups through `Q` and provides full map-like
// semantics for insertion, removal, iteration, and extension.
impl<K: Eq + Hash, V> KeyedStorageRead<K, V> for std::collections::HashMap<K, V> {
    type Iter<'a>
        = std::collections::hash_map::Iter<'a, K, V>
    where
        Self: 'a,
        K: 'a,
        V: 'a;
    
    type IterMut<'a>
        = std::collections::hash_map::IterMut<'a, K, V>
    where
        Self: 'a,
        K: 'a,
        V: 'a;

    fn get<Q>(&self, key: &Q) -> Result<Option<&V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.get(key))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.contains_key(key))
    }

    fn len(&self) -> Result<usize, StorageError> {
        Ok(self.len())
    }

    fn entries(&self) -> Result<Vec<(K, V)>, StorageError>
    where
        K: Clone,
        V: Clone,
    {
        Ok(self.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    fn keys(&self) -> Result<Vec<K>, StorageError>
    where
        K: Clone,
    {
        Ok(self.keys().cloned().collect())
    }

    fn values(&self) -> Result<Vec<V>, StorageError>
    where
        V: Clone,
    {
        Ok(self.values().cloned().collect())
    }

    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }
}

impl<K: Eq + Hash, V> KeyedStorageWrite<K, V> for std::collections::HashMap<K, V> {
    fn insert(&mut self, key: K, value: V) -> Result<Option<V>, StorageError> {
        Ok(self.insert(key, value))
    }

    fn try_insert(&mut self, key: K, value: V) -> Result<&V, StorageError> {
        use std::collections::hash_map::Entry;
        let vref = match self.entry(key) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(value),
        };
        Ok(&*vref)
    }

    fn try_insert_with<F>(&mut self, key: K, value: F) -> Result<&V, StorageError>
    where
        F: FnOnce() -> Result<V, StorageError>,
    {
        use std::collections::hash_map::Entry;
        let vref = match self.entry(key) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(value()?),
        };
        Ok(&*vref)
    }

    fn get_mut<Q>(&mut self, key: &Q) -> Result<Option<&mut V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.get_mut(key))
    }

    fn remove<Q>(&mut self, key: &Q) -> Result<Option<V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.remove(key))
    }

    fn clear(&mut self) -> Result<(), StorageError> {
        Ok(self.clear())
    }

    fn retain<F>(&mut self, f: F) -> Result<(), StorageError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        Ok(self.retain(f))
    }

    fn drain(&mut self) -> Result<Vec<(K, V)>, StorageError> {
        Ok(self.drain().collect())
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) -> Result<(), StorageError> {
        Ok(Extend::extend(self, iter))
    }

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.iter_mut()
    }
}

impl<K: Eq + Hash, V> KeyedStorage<K, V> for std::collections::HashMap<K, V> {
    fn new() -> Self {
        Self::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        FromIterator::from_iter(iter)
    }

    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> Result<&V, StorageError> {
        use std::collections::hash_map::Entry;
        let vref = match self.entry(key) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()),
        };
        Ok(&*vref)
    }

    fn get_or_try_insert_with<F: FnOnce() -> Result<V, StorageError>>(
        &mut self,
        key: K,
        f: F,
    ) -> Result<&V, StorageError> {
        use std::collections::hash_map::Entry;
        let vref = match self.entry(key) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()?),
        };
        Ok(&*vref)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{CowStorage, RwLockStorage};
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn hashmap_read_write() {
        let mut map: HashMap<String, u32> = HashMap::new();
        assert_eq!(
            *KeyedStorageWrite::try_insert(&mut map, "a".to_string(), 1).unwrap(),
            1
        );
        assert_eq!(
            *KeyedStorageWrite::try_insert(&mut map, "a".to_string(), 2).unwrap(),
            1
        );

        assert!(KeyedStorageRead::contains_key(&map, "a").unwrap());
        assert_eq!(*KeyedStorageRead::get(&map, "a").unwrap().unwrap(), 1);

        if let Some(value) = KeyedStorageWrite::get_mut(&mut map, &"a".to_string()).unwrap() {
            *value = 10;
        }
        assert_eq!(
            *KeyedStorageRead::get(&map, &"a".to_string())
                .unwrap()
                .unwrap(),
            10
        );

        let keys = KeyedStorageRead::keys(&map).unwrap();
        assert_eq!(keys, vec!["a".to_string()]);

        let values = KeyedStorageRead::values(&map).unwrap();
        assert_eq!(values, vec![10]);

        let entries = KeyedStorageRead::entries(&map).unwrap();
        assert_eq!(entries, vec![("a".to_string(), 10)]);

        let drained = KeyedStorageWrite::drain(&mut map).unwrap();
        assert_eq!(drained, vec![("a".to_string(), 10)]);
        assert!(KeyedStorageRead::is_empty(&map).unwrap());

        KeyedStorageWrite::extend(&mut map, vec![("x".to_string(), 1), ("y".to_string(), 2)])
            .unwrap();
        assert_eq!(KeyedStorageRead::len(&map).unwrap(), 2);
    }

    #[test]
    fn hashmap_edge_cases() {
        let mut map: HashMap<String, u32> = HashMap::new();

        assert_eq!(
            *KeyedStorageWrite::try_insert(&mut map, "a".to_string(), 1).unwrap(),
            1
        );
        assert_eq!(
            *KeyedStorageWrite::try_insert(&mut map, "a".to_string(), 2).unwrap(),
            1
        );
        assert_eq!(*KeyedStorageRead::get(&map, "a").unwrap().unwrap(), 1);

        let v = *KeyedStorage::get_or_insert_with(&mut map, "b".to_string(), || 10).unwrap();
        assert_eq!(v, 10);
        assert_eq!(*KeyedStorageRead::get(&map, "b").unwrap().unwrap(), 10);

        let res = KeyedStorage::get_or_try_insert_with(&mut map, "c".to_string(), || {
            Err::<u32, _>("fail".into())
        });
        assert!(res.is_err());
        assert_eq!(KeyedStorageRead::get(&map, "c").unwrap(), None);

        assert_eq!(
            *KeyedStorageWrite::try_insert(&mut map, "s".to_string(), 7).unwrap(),
            7
        );
        assert_eq!(*KeyedStorageRead::get(&map, "s").unwrap().unwrap(), 7);

        if let Some(v) = KeyedStorageWrite::get_mut(&mut map, "s").unwrap() {
            *v = 42;
        }
        assert_eq!(*KeyedStorageRead::get(&map, "s").unwrap().unwrap(), 42);
    }

    #[test]
    fn dyn_handle_read_write() {
        let boxed: Box<dyn KeyedDynHandle<_, _>> = Box::new(RwLockStorage::new(HashMap::new()));
        let key = "alpha".to_string();
        boxed.insert(key.clone(), 42).unwrap();
        assert_eq!(boxed.get(&key).unwrap().unwrap(), 42);
        assert!(boxed.contains_key(&key).unwrap());
        assert_eq!(boxed.len().unwrap(), 1);
        assert_eq!(boxed.keys().unwrap(), vec![key]);
    }

    #[test]
    fn dyn_handle_snapshots() {
        let cow = CowStorage::<HashMap<_, _>>::from_iter(vec![
            ("x".to_string(), 1),
            ("y".to_string(), 2),
        ]);
        let dyn_handle: &dyn KeyedDynHandle<_, _> = &cow;

        assert_eq!(dyn_handle.len().unwrap(), 2);
        assert!(!dyn_handle.is_empty().unwrap());

        let mut keys = dyn_handle.keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["x".to_string(), "y".to_string()]);

        let mut values = dyn_handle.values().unwrap();
        values.sort();
        assert_eq!(values, vec![1, 2]);

        let mut entries = dyn_handle.entries().unwrap();
        entries.sort_by_key(|(k, _)| k.clone());
        assert_eq!(entries, vec![("x".to_string(), 1), ("y".to_string(), 2),]);
    }

    #[test]
    fn dyn_handle_merge_drain_extend_retain_clear() {
        let rwl = RwLockStorage::new(HashMap::new());
        let cow = CowStorage::<HashMap<_, _>>::from_iter(vec![
            ("x".to_string(), 10),
            ("y".to_string(), 20),
        ]);

        let dyn1: &dyn KeyedDynHandle<_, _> = &cow;
        let dyn2: &dyn KeyedDynHandle<_, _> = &rwl;

        dyn1.merge(dyn2).unwrap();
        let drained = dyn1.drain().unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(dyn1.len().unwrap(), 0);

        dyn1.extend(&mut drained.into_iter()).unwrap();
        assert_eq!(dyn1.len().unwrap(), 2);
        assert_eq!(dyn1.get(&"x".to_string()).unwrap(), Some(10));
    }

    #[test]
    fn dyn_handle_lazy_initialization() {
        let rwl = RwLockStorage::new(HashMap::new());
        let dyn_handle: &dyn KeyedDynHandle<_, _> = &rwl;

        let value = dyn_handle
            .get_or_insert_with("key".to_string(), &mut || 100)
            .unwrap();
        assert_eq!(value, 100);
        assert_eq!(dyn_handle.get(&"key".to_string()).unwrap(), Some(100));

        let value = dyn_handle
            .get_or_insert_with("key".to_string(), &mut || 999)
            .unwrap();
        assert_eq!(value, 100);
    }

    #[test]
    fn dyn_handle_try_lazy_initialization() {
        let cow = CowStorage::new(HashMap::new());
        let dyn_handle: &dyn KeyedDynHandle<_, _> = &cow;

        let value = dyn_handle
            .get_or_try_insert_with("key".to_string(), &mut || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
    }

    #[test]
    fn dyn_handle_heterogeneous_storage() {
        let registries: Vec<Box<dyn KeyedDynHandle<_, _>>> = vec![
            Box::new(RwLockStorage::<HashMap<_, _>>::from_iter(vec![(
                "a".to_string(),
                1,
            )])),
            Box::new(CowStorage::<HashMap<_, _>>::from_iter(vec![(
                "b".to_string(),
                2,
            )])),
        ];

        assert_eq!(registries[0].get(&"a".to_string()).unwrap().unwrap(), 1);
        assert_eq!(registries[1].get(&"b".to_string()).unwrap().unwrap(), 2);
        assert_eq!(registries[0].get(&"b".to_string()).unwrap(), None);
    }
}
