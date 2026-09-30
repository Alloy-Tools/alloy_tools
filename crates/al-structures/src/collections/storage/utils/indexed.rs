use super::{HandleError, StorageError};
use std::{borrow::Borrow, ops::ControlFlow};

// ----- Read operations -----
/// Read-only operations for indexed storages.
pub trait IndexedHandleRead<V> {
    type Key;

    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&V) -> R;
    /// Run `f` on every borrowed key/value pair.
    fn for_each<F: FnMut((Self::Key, &V))>(&self, f: F) -> Result<(), HandleError>;
    /// Optional fallible variant for early exit / error propagation.
    fn try_for_each<F: FnMut((Self::Key, &V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<ControlFlow<B>, HandleError>;
    /// Accumulate a value over every borrowed key/value pair.
    fn fold<F: FnMut(A, (Self::Key, &V)) -> A, A>(
        &self,
        init: A,
        mut f: F,
    ) -> Result<A, HandleError> {
        let mut acc = Some(init);
        self.for_each(|pair| acc = Some(f(acc.take().unwrap(), pair)))?;
        Ok(acc.unwrap())
    }
    /// Collect one output for every borrowed key/value pair.
    fn map<F: FnMut((Self::Key, &V)) -> R, R>(&self, mut f: F) -> Result<Vec<R>, HandleError> {
        let mut out = Vec::new();
        self.for_each(|pair| out.push(f(pair)))?;
        Ok(out)
    }
    /// Returns the key of the first key/value pair to satisfy `f`, if any.
    fn find_key<F>(&self, mut f: F) -> Result<Option<Self::Key>, HandleError>
    where
        F: FnMut((&Self::Key, &V)) -> bool,
    {
        if let ControlFlow::Break(key) = self.try_for_each(|(k, v)| {
            if f((&k, v)) {
                ControlFlow::Break(k)
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
    fn find<F>(&self, mut f: F) -> Result<Option<(Self::Key, V)>, HandleError>
    where
        V: Clone,
        F: FnMut((&Self::Key, &V)) -> bool,
    {
        if let ControlFlow::Break(pair) = self.try_for_each(|(k, v)| {
            if f((&k, v)) {
                ControlFlow::Break((k, v.clone()))
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
        Q: Borrow<Self::Key> + Eq + ?Sized;
    /// Returns the number of entries in the storage.
    fn len(&self) -> Result<usize, HandleError>;
    /// Returns `true` if the storage is empty.
    fn is_empty(&self) -> Result<bool, HandleError> {
        self.len().map(|n| n == 0)
    }
    /// Returns a snapshot of all key‑value pairs.
    fn entries(&self) -> Result<Vec<(Self::Key, V)>, HandleError>
    where
        Self::Key: Clone,
        V: Clone;
    /// Returns a snapshot of all keys.
    fn keys(&self) -> Result<Vec<Self::Key>, HandleError>
    where
        Self::Key: Clone;
    /// Returns a snapshot of all values.
    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone;
}

/// Convenience extension methods for `IndexedHandleRead`.
///
/// These methods call the fallible versions on `IndexedHandleRead` and panic on error.
pub trait IndexedHandleReadExt<V>: IndexedHandleRead<V> {
    /// Infallible `get` - panics on error.
    fn get_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        V: Clone,
    {
        self.get(key).expect("`get` failed")
    }

    /// Infallible `with_ref` - panics on error.
    fn with_ref_unwrap<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        self.with_ref(key, f).expect("`with_ref` failed")
    }

    /// Infallible `for_each` - panics on error.
    fn for_each_unwrap<F: FnMut((Self::Key, &V))>(&self, f: F) {
        self.for_each(f).expect("`for_each` failed");
    }

    /// Infallible `try_for_each` - panics on error.
    fn try_for_each_unwrap<F: FnMut((Self::Key, &V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> ControlFlow<B> {
        self.try_for_each(f).expect("`try_for_each` failed")
    }

    /// Infallible `fold` - panics on error.
    fn fold_unwrap<F: FnMut(A, (Self::Key, &V)) -> A, A>(&self, init: A, f: F) -> A {
        self.fold(init, f).expect("`fold` failed")
    }

    /// Infallible `map` - panics on error.
    fn map_unwrap<F: FnMut((Self::Key, &V)) -> R, R>(&self, f: F) -> Vec<R> {
        self.map(f).expect("`map` failed")
    }

    /// Infallible `find_key` - panics on error.
    fn find_key_unwrap<F>(&self, f: F) -> Option<Self::Key>
    where
        F: FnMut((&Self::Key, &V)) -> bool,
    {
        self.find_key(f).expect("`find_key` failed")
    }

    /// Infallible `find` - panics on error.
    fn find_unwrap<F>(&self, f: F) -> Option<(Self::Key, V)>
    where
        V: Clone,
        F: FnMut((&Self::Key, &V)) -> bool,
    {
        self.find(f).expect("`find` failed")
    }

    /// Infallible `contains_key` - panics on error.
    fn contains_key_unwrap<Q>(&self, key: &Q) -> bool
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
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
    fn entries_unwrap(&self) -> Vec<(Self::Key, V)>
    where
        Self::Key: Clone,
        V: Clone,
    {
        self.entries().expect("`entries` failed")
    }

    /// Infallible snapshot of keys - panics on error.
    fn keys_unwrap(&self) -> Vec<Self::Key>
    where
        Self::Key: Clone,
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
impl<T, V> IndexedHandleReadExt<V> for T where T: IndexedHandleRead<V> {}

// ----- Write operations -----
/// Write operations for indexed storages.
pub trait IndexedHandleWrite<V>: IndexedHandleRead<V> {
    /// Insert a value into the storage and receive the paired key.
    fn push(&self, value: V) -> Result<Self::Key, HandleError>;
    /// Insert a value into the storage and receive an owned key/value pair.
    fn push_clone(&self, value: V) -> Result<(Self::Key, V), HandleError>
    where
        V: Clone;
    /// Insert a value at the given key, returning the old value if present.
    fn insert(&self, index: Self::Key, value: V) -> Result<Option<V>, HandleError>;
    /// Get an owned value from the key, inserting the passed value if none exists.
    fn try_insert(&self, index: Self::Key, value: V) -> Result<V, HandleError>
    where
        V: Clone;
    /// Get an owned value from the key, attempting to insert a value if none exists using `f`.
    fn try_insert_with<F>(&self, key: Self::Key, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>;
    /// Mutate the value for `key` via closure `f`. Returns the closure's result if present.
    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&mut V) -> R;
    /// Run `f` on every mutable key/value pair.
    fn for_each_mut<F: FnMut((Self::Key, &mut V))>(&self, f: F) -> Result<(), HandleError>;
    /// Optional fallible mutable variant.
    fn try_for_each_mut<F: FnMut((Self::Key, &mut V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<ControlFlow<B>, HandleError>;
    /// Accumulate a value over every key/value pair.
    fn fold_mut<F: FnMut(A, (Self::Key, &mut V)) -> A, A>(
        &self,
        init: A,
        mut f: F,
    ) -> Result<A, HandleError> {
        let mut acc = Some(init);
        self.for_each_mut(|pair| acc = Some(f(acc.take().unwrap(), pair)))?;
        Ok(acc.unwrap())
    }
    /// Collect one output for every key/value pair.
    fn map_mut<F: FnMut((Self::Key, &mut V)) -> R, R>(
        &self,
        mut f: F,
    ) -> Result<Vec<R>, HandleError> {
        let mut out = Vec::new();
        self.for_each_mut(|pair| out.push(f(pair)))?;
        Ok(out)
    }
    /// Remove and return any existing entry under the key.
    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized;
    /// Remove all entries.
    fn clear(&self) -> Result<(), HandleError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut((Self::Key, &V)) -> bool;
    //REVIEW: Should drain take a range?
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(Self::Key, V)>, HandleError>;
    /// Insert all entries from an iterator.
    fn extend<I: IntoIterator<Item = V>>(&self, iter: I) -> Result<Vec<Self::Key>, HandleError>;
    /// Merge another storage into this one.
    fn merge<R>(&self, other: &R) -> Result<Vec<Self::Key>, HandleError>
    where
        R: IndexedHandleRead<V>,
        V: Clone,
    {
        self.extend(other.values()?)
    }
}

/// Convenience extension methods for `IndexedHandleWrite`.
///
/// These methods call the fallible versions on `IndexedHandleWrite` and panic on error.
pub trait IndexedHandleWriteExt<V>: IndexedHandleWrite<V> {
    /// Infallible `push` - panics on error.
    fn push_unwrap(&self, value: V) -> Self::Key {
        self.push(value).expect("`push` failed")
    }

    /// Infallible `push_clone` - panics on error.
    fn push_clone_unwrap(&self, value: V) -> (Self::Key, V)
    where
        V: Clone,
    {
        self.push_clone(value).expect("`push_clone` failed")
    }

    /// Infallible `insert` - panics on error.
    fn insert_unwrap(&self, index: Self::Key, value: V) -> Option<V> {
        self.insert(index, value).expect("`insert` failed")
    }

    /// Infallible `try_insert` - panics on error.
    fn try_insert_unwrap(&self, key: Self::Key, value: V) -> V
    where
        V: Clone,
    {
        self.try_insert(key, value).expect("`try_insert` failed")
    }

    /// Infallible `try_insert_with` - panics on error.
    fn try_insert_with_unwrap<F>(&self, key: Self::Key, value: F) -> V
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        self.try_insert_with(key, value)
            .expect("`try_insert_with` failed")
    }

    /// Infallible `with_mut` - panics on error.
    fn with_mut_unwrap<Q, F, R>(&self, index: &Q, f: F) -> Option<R>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        self.with_mut(index, f).expect("`with_mut` failed")
    }

    /// Infallible `for_each_mut` - panics on error.
    fn for_each_mut_unwrap<F: FnMut((Self::Key, &mut V))>(&self, f: F) {
        self.for_each_mut(f).expect("`for_each_mut` failed")
    }

    /// Infallible `try_for_each_mut` - panics on error.
    fn try_for_each_mut_unwrap<F: FnMut((Self::Key, &mut V)) -> ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> ControlFlow<B> {
        self.try_for_each_mut(f).expect("`try_for_each_mut` failed")
    }

    /// Infallible `fold_mut` - panics on error.
    fn fold_mut_unwrap<F: FnMut(A, (Self::Key, &mut V)) -> A, A>(&self, init: A, f: F) -> A {
        self.fold_mut(init, f).expect("`fold_mut` failed")
    }

    /// Infallible `map_mut` - panics on error.
    fn map_mut_unwrap<F: FnMut((Self::Key, &mut V)) -> R, R>(&self, f: F) -> Vec<R> {
        self.map_mut(f).expect("`map_mut` failed")
    }

    /// Infallible `remove` - panics on error.
    fn remove_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
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
        F: FnMut((Self::Key, &V)) -> bool,
    {
        self.retain(f).expect("`retain` failed")
    }

    /// Infallible `drain` - panics on error.
    fn drain_unwrap(&self) -> Vec<(Self::Key, V)> {
        self.drain().expect("`drain` failed")
    }

    /// Infallible `extend` - panics on error.
    fn extend_unwrap<I>(&self, iter: I) -> Vec<Self::Key>
    where
        I: IntoIterator<Item = V>,
    {
        self.extend(iter).expect("`extend` failed")
    }

    /// Infallible `merge` - panics on error.
    fn merge_unwrap<R>(&self, other: &R) -> Vec<Self::Key>
    where
        R: IndexedHandleRead<V>,
        V: Clone,
    {
        self.merge(other).expect("`merge` failed")
    }
}

impl<T, V> IndexedHandleWriteExt<V> for T where T: IndexedHandleWrite<V> {}

// ----- Read-Write union -----
/// A complete read-write indexed storage interface.
///
/// This trait combines `IndexedHandleRead` and `IndexedHandleWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait IndexedHandle<V>: IndexedHandleWrite<V> {
    type Storage;
    fn new(storage: Self::Storage) -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the key and value for `key` if it exists; otherwise call
    /// `f`, insert the result, and return the generated key and value.
    fn get_or_insert_with<F>(&self, key: Self::Key, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> V;

    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with<F>(&self, key: Self::Key, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>;
}

// ----- Dyn-Compatible wrapper -----
/// Dyn-Compatible indexed storage trait.
///
/// This mirrors `IndexedHandle<V>` but erases generic closures/iterators so the
/// handle can be used behind trait objects (for example, storing different
/// handle implementations in a homogeneous collection).
///
/// # Examples
///
/// ```
/// use al_structures::collections::storage::{RwLockStorage, utils::indexed::IndexedDynHandle};
///
/// let boxed: Box<dyn IndexedDynHandle<String, Key = u8>> = Box::new(RwLockStorage::new(Vec::new()));
///
/// boxed.push("x".to_string()).unwrap();
/// assert_eq!(boxed.get(&0).unwrap().unwrap(), "x".to_string());
/// ```
pub trait IndexedDynHandle<V>: Send + Sync + 'static {
    type Key;
    // ----- reads -----
    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get(&self, key: &Self::Key) -> Result<Option<V>, HandleError>
    where
        Self::Key: Eq,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref(&self, key: &Self::Key, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        Self::Key: Eq;
    /// Run `f` on every borrowed key/value pair.
    fn for_each(&self, f: &mut dyn FnMut((Self::Key, &V))) -> Result<(), HandleError>;
    /// Optional fallible variant for early exit / error propagation.
    fn try_for_each(
        &self,
        f: &mut dyn FnMut((Self::Key, &V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError>;
    // Returns the key of the first key/value pair to satisfy `f`, if any.
    fn find_key(
        &self,
        f: &mut dyn FnMut((&Self::Key, &V)) -> bool,
    ) -> Result<Option<Self::Key>, HandleError>;
    // Returns the first key/value pair to satisfy `f`, if any.
    fn find(
        &self,
        f: &mut dyn FnMut((&Self::Key, &V)) -> bool,
    ) -> Result<Option<(Self::Key, V)>, HandleError>
    where
        Self::Key: Clone,
        V: Clone;
    /// Returns `true` if the storage contains the key.
    fn contains_key(&self, key: &Self::Key) -> Result<bool, HandleError>
    where
        Self::Key: Eq;
    /// Returns the number of entries in the storage.
    fn len(&self) -> Result<usize, HandleError>;
    /// Returns `true` if the storage is empty.
    fn is_empty(&self) -> Result<bool, HandleError> {
        self.len().map(|l| l == 0)
    }
    /// Returns a snapshot of all key‑value pairs.
    fn entries(&self) -> Result<Vec<(Self::Key, V)>, HandleError>
    where
        Self::Key: Clone,
        V: Clone;
    /// Returns a snapshot of all keys.
    fn keys(&self) -> Result<Vec<Self::Key>, HandleError>
    where
        Self::Key: Clone;
    /// Returns a snapshot of all values.
    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone;

    // ----- writes -----
    /// Insert a value into the storage and receive the paired key.
    fn push(&self, value: V) -> Result<Self::Key, HandleError>;
    /// Insert a value into the storage and receive an owned key/value pair.
    fn push_clone(&self, value: V) -> Result<(Self::Key, V), HandleError>
    where
        V: Clone;
    /// Insert the passed key/value pair into the storage, returning any old value.
    fn insert(&self, index: Self::Key, value: V) -> Result<Option<V>, HandleError>;
    /// Get an owned value from the key, inserting the passed value if none exists.
    fn try_insert(&self, index: Self::Key, value: V) -> Result<V, HandleError>
    where
        V: Clone;
    /// Get an owned value from the key, attempting to insert a value if none exists using `f`.
    fn try_insert_with(
        &self,
        key: Self::Key,
        value: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone;
    /// Mutate the value for `key` via closure `f`. Returns the closure's result if present.
    fn with_mut(&self, key: &Self::Key, f: &mut dyn FnMut(&mut V)) -> Result<bool, HandleError>
    where
        Self::Key: Eq;
    /// Run `f` on every mutable key/value pair.
    fn for_each_mut(&self, f: &mut dyn FnMut((Self::Key, &mut V))) -> Result<(), HandleError>;
    /// Optional fallible mutable variant.
    fn try_for_each_mut(
        &self,
        f: &mut dyn FnMut((Self::Key, &mut V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError>;
    /// Remove and return any existing entry under the key.
    fn remove(&self, key: &Self::Key) -> Result<Option<V>, HandleError>
    where
        Self::Key: Eq;
    /// Remove all entries.
    fn clear(&self) -> Result<(), HandleError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain(&self, f: &mut dyn FnMut((Self::Key, &V)) -> bool) -> Result<(), HandleError>;
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(Self::Key, V)>, HandleError>;
    /// Insert all entries from an iterator.
    fn extend(&self, iter: &mut dyn Iterator<Item = V>) -> Result<Vec<Self::Key>, HandleError>;

    /// Merge another `IndexedDynHandle` into this one.
    fn merge(
        &self,
        other: &dyn IndexedDynHandle<V, Key = Self::Key>,
    ) -> Result<Vec<Self::Key>, HandleError>
    where
        V: Clone + 'static,
    {
        self.extend(&mut other.values()?.into_iter())
    }

    // ---- lazy initialisation ----
    /// Return the value for `key` if it exists; otherwise call `f` and insert the result.
    fn get_or_insert_with(
        &self,
        key: Self::Key,
        f: &mut dyn FnMut() -> V,
    ) -> Result<V, HandleError>
    where
        V: Clone;
    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with(
        &self,
        key: Self::Key,
        f: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone;
}

impl<K, V, T: IndexedHandle<V, Key = K> + Send + Sync + 'static> IndexedDynHandle<V> for T {
    type Key = K;

    fn get(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Eq,
        V: Clone,
    {
        self.get(key)
    }

    fn with_ref(&self, key: &K, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        K: Eq,
    {
        self.with_ref(key, f).map(|o| o.is_some())
    }

    fn for_each(&self, f: &mut dyn FnMut((Self::Key, &V))) -> Result<(), HandleError> {
        self.for_each(f)
    }

    fn try_for_each(
        &self,
        f: &mut dyn FnMut((Self::Key, &V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError> {
        self.try_for_each(f)
    }

    fn find_key(
        &self,
        f: &mut dyn FnMut((&Self::Key, &V)) -> bool,
    ) -> Result<Option<K>, HandleError> {
        self.find_key(f)
    }

    fn find(
        &self,
        f: &mut dyn FnMut((&Self::Key, &V)) -> bool,
    ) -> Result<Option<(K, V)>, HandleError>
    where
        Self::Key: Clone,
        V: Clone,
    {
        self.find(f)
    }

    fn contains_key(&self, key: &K) -> Result<bool, HandleError>
    where
        K: Eq,
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

    fn push(&self, value: V) -> Result<K, HandleError> {
        self.push(value)
    }

    fn push_clone(&self, value: V) -> Result<(K, V), HandleError>
    where
        V: Clone,
    {
        self.push_clone(value)
    }

    fn insert(&self, index: K, value: V) -> Result<Option<V>, HandleError> {
        self.insert(index, value)
    }

    fn try_insert(&self, index: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.try_insert(index, value)
    }

    fn try_insert_with(
        &self,
        key: Self::Key,
        value: &mut dyn FnMut() -> Result<V, StorageError>,
    ) -> Result<V, HandleError>
    where
        V: Clone,
    {
        self.try_insert_with(key, value)
    }

    fn with_mut(&self, index: &K, f: &mut dyn FnMut(&mut V)) -> Result<bool, HandleError>
    where
        K: Eq,
    {
        self.with_mut(index, f).map(|o| o.is_some())
    }

    fn for_each_mut(&self, f: &mut dyn FnMut((Self::Key, &mut V))) -> Result<(), HandleError> {
        self.for_each_mut(f)
    }

    fn try_for_each_mut(
        &self,
        f: &mut dyn FnMut((Self::Key, &mut V)) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, HandleError> {
        self.try_for_each_mut(f)
    }

    fn remove(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Eq,
    {
        self.remove(key)
    }

    fn clear(&self) -> Result<(), HandleError> {
        self.clear()
    }

    fn retain(&self, f: &mut dyn FnMut((Self::Key, &V)) -> bool) -> Result<(), HandleError> {
        self.retain(f)
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        self.drain()
    }

    fn extend(&self, iter: &mut dyn Iterator<Item = V>) -> Result<Vec<K>, HandleError> {
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
/// Read-only indexed container behavior.
///
/// `IndexedStorageRead` is designed for append-style backends whose values are
/// accessed by generated keys, typically numeric indices.
pub trait IndexedStorageRead<V> {
    type Key;
    type Iter<'a>: Iterator<Item = (Self::Key, &'a V)> + 'a
    where
        Self: 'a,
        V: 'a;
    type IterMut<'a>: Iterator<Item = (Self::Key, &'a mut V)> + 'a
    where
        V: 'a;

    /// Returns a reference to the value associated with the generated key.
    fn get<Q: Borrow<Self::Key> + ?Sized>(&self, key: &Q) -> Result<Option<&V>, StorageError>;

    /// Returns whether a value exists for the generated key.
    fn contains_key<Q: Borrow<Self::Key> + ?Sized>(&self, key: &Q) -> Result<bool, StorageError>;

    /// Returns the number of stored values.
    fn len(&self) -> Result<usize, StorageError>;

    /// Returns whether the container is empty.
    fn is_empty(&self) -> Result<bool, StorageError> {
        self.len().map(|l| l == 0)
    }

    /// Returns owned key/value pairs for all entries.
    fn entries(&self) -> Result<Vec<(Self::Key, V)>, StorageError>
    where
        Self::Key: Clone,
        V: Clone;

    /// Returns owned keys for all entries.
    fn keys(&self) -> Result<Vec<Self::Key>, StorageError>
    where
        Self::Key: Clone;

    /// Returns owned values for all entries.
    fn values(&self) -> Result<Vec<V>, StorageError>
    where
        V: Clone;

    /// Returns an iterator over generated keys and borrowed values.
    fn iter(&self) -> Self::Iter<'_>;
}

/// Mutable indexed storage behavior for append-style backends.
///
/// `IndexedStorageWrite` extends `IndexedStorageRead` with insertion,
/// replacement, and removal operations.
pub trait IndexedStorageWrite<V>: IndexedStorageRead<V> + Default {
    /// Append a value and return the generated key.
    fn push(&mut self, value: V) -> Result<Self::Key, StorageError>;

    /// Insert a value at the given key, returning the old value if present.
    fn insert(&mut self, index: Self::Key, value: V) -> Result<Option<V>, StorageError>;

    /// Insert a value only when the key is vacant.
    fn try_insert(&mut self, index: Self::Key, value: V) -> Result<&V, StorageError>;

    /// Insert a value only when the key is vacant.
    fn try_insert_with<F>(&mut self, index: Self::Key, value: F) -> Result<&V, StorageError>
    where
        F: FnOnce() -> Result<V, StorageError>;

    /// Returns a mutable reference to the value at `key`, if present.
    fn get_mut<Q: Borrow<Self::Key> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Result<Option<&mut V>, StorageError>;

    /// Removes and returns the value indexed by `key`, if it existed.
    fn remove<Q: Borrow<Self::Key> + ?Sized>(&mut self, key: &Q)
        -> Result<Option<V>, StorageError>;

    /// Clears all values from the indexed container.
    fn clear(&mut self) -> Result<(), StorageError>;

    /// Retains only values that satisfy the predicate.
    fn retain<F: FnMut((Self::Key, &V)) -> bool>(&mut self, f: F) -> Result<(), StorageError>;

    /// Removes a range of values and returns them as owned items.
    fn drain<R: std::ops::RangeBounds<Self::Key>>(
        &mut self,
        range: R,
    ) -> Result<Vec<(Self::Key, V)>, StorageError>;

    /// Extends the container with owned values from `iter`.
    fn extend<I: IntoIterator<Item = V>>(
        &mut self,
        iter: I,
    ) -> Result<Vec<Self::Key>, StorageError>;

    /// Merge another storage into this one.
    fn merge<R>(&mut self, other: &R) -> Result<Vec<Self::Key>, StorageError>
    where
        R: IndexedStorageRead<V>,
        V: Clone,
    {
        self.extend(other.values()?)
    }

    /// Returns an iterator over borrowed mutable entries.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

/// A complete read-write indexed storage interface.
///
/// This trait combines `IndexedStorageRead` and `IndexedStorageWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait IndexedStorage<V>: IndexedStorageWrite<V> {
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the key and value for `key` if it exists; otherwise call
    /// `f`, insert the result, and return the generated key and value.
    fn get_or_insert_with<F: FnOnce() -> V>(
        &mut self,
        key: Self::Key,
        f: F,
    ) -> Result<&V, StorageError>;

    /// Fallible version of `get_or_insert_with`.
    fn get_or_try_insert_with<F: FnOnce() -> Result<V, StorageError>>(
        &mut self,
        key: Self::Key,
        f: F,
    ) -> Result<&V, StorageError>;
}

// ----- Vec storage -----
/// `Vec` storage implementation for indexed read/write behavior.
impl<V> IndexedStorageRead<V> for Vec<V> {
    type Key = usize;

    type Iter<'a>
        = std::iter::Enumerate<std::slice::Iter<'a, V>>
    where
        Self: 'a,
        V: 'a;

    type IterMut<'a>
        = std::iter::Enumerate<std::slice::IterMut<'a, V>>
    where
        V: 'a;

    fn get<Q: Borrow<Self::Key> + ?Sized>(&self, key: &Q) -> Result<Option<&V>, StorageError> {
        Ok(self.as_slice().get(*key.borrow()))
    }

    fn contains_key<Q: Borrow<Self::Key> + ?Sized>(&self, key: &Q) -> Result<bool, StorageError> {
        Ok(self.as_slice().get(*key.borrow()).is_some())
    }

    fn len(&self) -> Result<usize, StorageError> {
        Ok(self.as_slice().len())
    }

    fn entries(&self) -> Result<Vec<(Self::Key, V)>, StorageError>
    where
        Self::Key: Clone,
        V: Clone,
    {
        Ok(self.iter().map(|(i, v)| (i.clone(), v.clone())).collect())
    }

    fn keys(&self) -> Result<Vec<Self::Key>, StorageError>
    where
        Self::Key: Clone,
    {
        Ok((0..self.len()).collect())
    }

    fn values(&self) -> Result<Vec<V>, StorageError>
    where
        V: Clone,
    {
        Ok(self.as_slice().to_vec())
    }

    fn iter(&self) -> Self::Iter<'_> {
        self.as_slice().iter().enumerate()
    }
}

impl<V> IndexedStorageWrite<V> for Vec<V> {
    fn push(&mut self, value: V) -> Result<Self::Key, StorageError> {
        self.push(value);
        Ok(self.len() - 1)
    }

    fn insert(&mut self, index: Self::Key, value: V) -> Result<Option<V>, StorageError> {
        let len = self.len();
        if index < len {
            Ok(Some(std::mem::replace(&mut self[index], value)))
        } else if index == len {
            self.push(value);
            Ok(None)
        } else {
            Err(StorageError::OutOfBounds(index, len))
        }
    }

    fn try_insert(&mut self, index: Self::Key, value: V) -> Result<&V, StorageError> {
        let len = self.len();
        if index < len {
            self.get(&index)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Vec is missing value at index '{index}' which is less then the length '{len}'"
                ))
            })
        } else if index == len {
            self.push(value);
            self.get(&index)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Vec is missing value at index '{index}' which was just pushed"
                ))
            })
        } else {
            Err(StorageError::OutOfBounds(index, len))
        }
    }

    fn try_insert_with<F>(&mut self, index: Self::Key, value: F) -> Result<&V, StorageError>
    where
        F: FnOnce() -> Result<V, StorageError>,
    {
        let len = self.len();
        if index < len {
            self.get(&index)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Vec is missing value at index '{index}' which is less then the length '{len}'"
                ))
            })
        } else if index == len {
            self.push(value()?);
            self.get(&index)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Vec is missing value at index '{index}' which was just pushed"
                ))
            })
        } else {
            Err(StorageError::OutOfBounds(index, len))
        }
    }

    fn get_mut<Q: Borrow<Self::Key> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Result<Option<&mut V>, StorageError> {
        Ok(self.as_mut_slice().get_mut(*key.borrow()))
    }

    fn remove<Q: Borrow<Self::Key> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Result<Option<V>, StorageError> {
        let index = *key.borrow();
        if index < self.len() {
            Ok(Some(self.remove(index)))
        } else {
            Ok(None)
        }
    }

    fn clear(&mut self) -> Result<(), StorageError> {
        Ok(self.clear())
    }

    fn retain<F: FnMut((Self::Key, &V)) -> bool>(&mut self, mut f: F) -> Result<(), StorageError> {
        let mut index = 0;
        Ok(self.retain(|v| {
            let r = f((index, v));
            index += 1;
            r
        }))
    }

    fn drain<R: std::ops::RangeBounds<Self::Key>>(
        &mut self,
        range: R,
    ) -> Result<Vec<(Self::Key, V)>, StorageError> {
        Ok(self.drain(range).enumerate().collect())
    }

    fn extend<I: IntoIterator<Item = V>>(
        &mut self,
        iter: I,
    ) -> Result<Vec<Self::Key>, StorageError> {
        let first = self.len();
        Extend::extend(self, iter);
        Ok((first..self.len()).collect())
    }

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.as_mut_slice().iter_mut().enumerate()
    }
}

impl<V> IndexedStorage<V> for Vec<V> {
    fn new() -> Self {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        iter.into_iter().collect()
    }

    fn get_or_insert_with<F: FnOnce() -> V>(
        &mut self,
        key: Self::Key,
        f: F,
    ) -> Result<&V, StorageError> {
        let len = self.len();
        if key < len {
            Ok(self.get(&key)?.ok_or_else(|| {
                StorageError::MissingValue(format!("Missing value that should be at index '{key}'"))
            })?)
        } else if key == len {
            self.push(f());
            Ok(self.get(&key)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Missing value that was just pushed to index '{key}'"
                ))
            })?)
        } else {
            Err(StorageError::OutOfBounds(key, len))
        }
    }

    fn get_or_try_insert_with<F: FnOnce() -> Result<V, StorageError>>(
        &mut self,
        key: Self::Key,
        f: F,
    ) -> Result<&V, StorageError> {
        let len = self.len();
        if key < len {
            Ok(self.get(&key)?.ok_or_else(|| {
                StorageError::MissingValue(format!("Missing value that should be at index '{key}'"))
            })?)
        } else if key == len {
            self.push(f()?);
            Ok(self.get(&key)?.ok_or_else(|| {
                StorageError::MissingValue(format!(
                    "Missing value that was just pushed to index '{key}'"
                ))
            })?)
        } else {
            Err(StorageError::OutOfBounds(key, len))
        }
    }
}

// ----- StableVec storage -----
//REVIEW: impl for `StableVec` after generation changes

#[cfg(test)]
mod tests {
    use super::super::super::{CowStorage, RwLockStorage};
    use super::*;

    #[test]
    fn vec_read_and_write() {
        let mut vec: Vec<u32> = Vec::new();
        // Appending values yields generated keys.
        assert_eq!(IndexedStorageWrite::push(&mut vec, 10).unwrap(), 0);
        assert_eq!(IndexedStorageWrite::push(&mut vec, 20).unwrap(), 1);

        // Basic lookup and membership checks.
        assert_eq!(*IndexedStorageRead::get(&vec, &0).unwrap().unwrap(), 10);
        assert!(IndexedStorageRead::contains_key(&vec, &1).unwrap());
        assert!(!IndexedStorageRead::contains_key(&vec, &2).unwrap());

        // Keys and values reflect current contents.
        assert_eq!(IndexedStorageRead::keys(&vec).unwrap(), vec![0, 1]);
        assert_eq!(IndexedStorageRead::values(&vec).unwrap(), vec![10, 20]);

        // try_insert appends only when index equals len, and reports occupancy otherwise.
        assert_eq!(
            *IndexedStorageWrite::try_insert(&mut vec, 2, 30).unwrap(),
            30
        );
        assert_eq!(
            *IndexedStorageWrite::try_insert(&mut vec, 1, 40).unwrap(),
            20
        );

        // insert at existing index replaces and returns old value.
        assert_eq!(
            IndexedStorageWrite::insert(&mut vec, 1, 25)
                .unwrap()
                .unwrap(),
            20
        );
        assert_eq!(*IndexedStorageRead::get(&vec, &1).unwrap().unwrap(), 25);

        // Mutate an element via `get_mut` and verify.
        if let Some(value) = IndexedStorageWrite::get_mut(&mut vec, &0).unwrap() {
            *value = 15;
        }
        assert_eq!(*IndexedStorageRead::get(&vec, &0).unwrap().unwrap(), 15);

        // Removing by index returns the removed element and shifts following elements.
        let removed = IndexedStorageWrite::remove(&mut vec, &2).unwrap().unwrap();
        assert_eq!(removed, 30);
        assert_eq!(IndexedStorageRead::len(&vec).unwrap(), 2);

        // Drain all elements and verify container emptiness.
        let drained = IndexedStorageWrite::drain(&mut vec, ..).unwrap();
        assert_eq!(drained, vec![(0, 15), (1, 25)]);
        assert!(IndexedStorageRead::is_empty(&vec).unwrap());

        // Extend with new elements and confirm keys reset from 0.
        IndexedStorageWrite::extend(&mut vec, vec![100, 200]).unwrap();
        assert_eq!(IndexedStorageRead::keys(&vec).unwrap(), vec![0usize, 1]);
    }

    #[test]
    fn vec_edge_cases() {
        let mut vec: Vec<u32> = Vec::new();

        // push returns incremental keys
        assert_eq!(IndexedStorageWrite::push(&mut vec, 5).unwrap(), 0);
        assert_eq!(IndexedStorageWrite::push(&mut vec, 6).unwrap(), 1);

        // insert at len appends
        assert_eq!(IndexedStorageWrite::insert(&mut vec, 2, 7).unwrap(), None);
        assert_eq!(IndexedStorageRead::len(&vec).unwrap(), 3);

        // insert out of bounds errors
        match IndexedStorageWrite::insert(&mut vec, 5, 9) {
            Err(StorageError::OutOfBounds(i, l)) => {
                assert_eq!(i, 5);
                assert_eq!(l, 3);
            }
            other => panic!("expected OutOfBounds, got {:?}", other),
        }

        // try_insert behaviour
        assert_eq!(
            *IndexedStorageWrite::try_insert(&mut vec, 1, 10).unwrap(),
            6
        );
        assert_eq!(
            *IndexedStorageWrite::try_insert(&mut vec, 3, 11).unwrap(),
            11
        );

        // get_or_insert_with returns existing key/value when present
        assert_eq!(
            *IndexedStorage::get_or_insert_with(&mut vec, 1, || 99).unwrap(),
            6
        );

        // get_or_insert_with pushes when absent and returns new key/value
        let v2 = {
            let v2 = *IndexedStorage::get_or_insert_with(&mut vec, 4, || 100).unwrap();
            assert_eq!(v2, 100);
            v2
        };
        assert_eq!(*IndexedStorageRead::get(&vec, &4).unwrap().unwrap(), v2);

        // get_or_try_insert_with error does not push
        let before = IndexedStorageRead::len(&vec).unwrap();
        let res =
            IndexedStorage::get_or_try_insert_with(&mut vec, 20, || Err::<u32, _>("err".into()));
        assert!(res.is_err());
        assert_eq!(IndexedStorageRead::len(&vec).unwrap(), before);
    }

    #[test]
    fn dyn_handle_read_write() {
        let boxed: Box<dyn IndexedDynHandle<_, Key = _>> = Box::new(RwLockStorage::new(Vec::new()));
        let k1 = boxed.push(42).unwrap();
        let k2 = boxed.push(24).unwrap();
        assert_eq!(boxed.get(&k1).unwrap().unwrap(), 42);
        assert!(boxed.contains_key(&k2).unwrap());
        assert_eq!(boxed.len().unwrap(), 2);
        assert_eq!(boxed.keys().unwrap(), vec![0, 1]);
    }

    #[test]
    fn dyn_handle_snapshots() {
        let cow = CowStorage::<Vec<_>>::from_iter(vec!["x".to_string(), "y".to_string()]);
        let dyn_handle: &dyn IndexedDynHandle<_, Key = _> = &cow;

        assert_eq!(dyn_handle.len().unwrap(), 2);
        assert!(!dyn_handle.is_empty().unwrap());

        let mut keys = dyn_handle.keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec![0, 1]);

        let mut values = dyn_handle.values().unwrap();
        values.sort();
        assert_eq!(values, vec!["x".to_string(), "y".to_string()]);

        let mut entries = dyn_handle.entries().unwrap();
        entries.sort_by_key(|(k, _)| k.clone());
        assert_eq!(entries, vec![(0, "x".to_string()), (1, "y".to_string()),]);
    }

    #[test]
    fn dyn_handle_merge_drain_extend_retain_clear() {
        let rwl = RwLockStorage::new(Vec::new());
        let cow = CowStorage::<Vec<_>>::from_iter(vec!["x".to_string(), "y".to_string()]);

        let dyn1: &dyn IndexedDynHandle<_, Key = _> = &cow;
        let dyn2: &dyn IndexedDynHandle<_, Key = _> = &rwl;

        dyn1.merge(dyn2).unwrap();
        let drained = dyn1.drain().unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(dyn1.len().unwrap(), 0);

        dyn1.extend(&mut drained.into_iter().map(|(_, v)| v))
            .unwrap();
        assert_eq!(dyn1.len().unwrap(), 2);
        assert_eq!(dyn1.get(&0).unwrap().unwrap(), "x".to_string());
    }

    #[test]
    fn dyn_handle_lazy_initialization() {
        let rwl = RwLockStorage::new(Vec::new());
        let dyn_handle: &dyn IndexedDynHandle<_, Key = _> = &rwl;

        let value = dyn_handle
            .get_or_insert_with(0, &mut || "value".to_string())
            .unwrap();
        assert_eq!(value, "value".to_string());
        assert_eq!(dyn_handle.get(&0).unwrap().unwrap(), "value".to_string());

        let value = dyn_handle
            .get_or_insert_with(0, &mut || "new_value".to_string())
            .unwrap();
        assert_eq!(value, "value".to_string());
    }

    #[test]
    fn dyn_handle_try_lazy_initialization() {
        let cow = CowStorage::new(Vec::new());
        let dyn_handle: &dyn IndexedDynHandle<_, Key = _> = &cow;

        let value = dyn_handle
            .get_or_try_insert_with(0, &mut || Ok("value".to_string()))
            .unwrap();
        assert_eq!(value, "value".to_string());
    }

    #[test]
    fn dyn_handle_heterogeneous_storage() {
        let registries: Vec<Box<dyn IndexedDynHandle<_, Key = _>>> = vec![
            Box::new(RwLockStorage::<Vec<_>>::from_iter(vec!["a".to_string()])),
            Box::new(CowStorage::<Vec<_>>::from_iter(vec!["b".to_string()])),
        ];

        assert_eq!(registries[0].get(&0).unwrap().unwrap(), "a".to_string());
        assert_eq!(registries[1].get(&0).unwrap().unwrap(), "b".to_string());
        assert_eq!(registries[0].get(&1).unwrap(), None);
        assert_eq!(registries[1].get(&1).unwrap(), None);
    }
}
