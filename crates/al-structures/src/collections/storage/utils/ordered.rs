use super::{HandleError, StorageError};
use std::{borrow::Borrow, ops::ControlFlow};

// ----- Read operations -----
/// Read-only operations for ordered storages.
pub trait OrderedHandleRead<K, V> {
    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
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
        Q: Ord + ?Sized;
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

/// Convenience extension methods for `OrderedHandleRead`.
///
/// These methods call the fallible versions on `OrderedHandleRead` and panic on error.
pub trait OrderedHandleReadExt<K, V>: OrderedHandleRead<K, V> {
    /// Infallible `get` - panics on error.
    fn get_unwrap<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Clone,
    {
        self.get(key).expect("`get` failed")
    }

    /// Infallible `with_ref` - panics on error.
    fn with_ref_unwrap<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
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
        Q: Ord + ?Sized,
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
impl<T, K, V> OrderedHandleReadExt<K, V> for T where T: OrderedHandleRead<K, V> {}

// ----- Write operarions -----
/// Write operations for ordered storages.
pub trait OrderedHandleWrite<K, V>: OrderedHandleRead<K, V> {
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
        Q: Ord + ?Sized,
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
        Q: Ord + ?Sized;
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
    /// Merge another storage into this one.
    fn merge<R>(&self, other: &R) -> Result<(), HandleError>
    where
        R: OrderedHandleRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.extend(other.entries()?)
    }
}

/// Convenience extension methods for `OrderedHandleWrite`.
///
/// These methods call the fallible versions on `OrderedHandleWrite` and panic on error.
pub trait OrderedHandleWriteExt<K, V>: OrderedHandleWrite<K, V> {
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
    fn with_mut_unwrap<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
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
        Q: Ord + ?Sized,
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
        R: OrderedHandleRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.merge(other).expect("`merge` failed")
    }
}

// Blanket implementation
impl<T, K, V> OrderedHandleWriteExt<K, V> for T where T: OrderedHandleWrite<K, V> {}

// ----- Read-Write union -----
/// A complete read-write ordered storage interface.
///
/// This trait combines `OrderedHandleRead` and `OrderedHandleWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait OrderedHandle<K, V>: OrderedHandleWrite<K, V> {
    type Storage;
    fn new(storage: Self::Storage) -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self;

    // ----- lazy initialisation -----
    /// Return the value for `key` if it exists; otherwise call `f`,
    /// insert the result, and return a clone.
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
/// Dyn-Compatible ordered storage trait.
///
/// This mirrors `OrderedHandle<K,V>` but erases generic closures/iterators so the
/// handle can be used behind trait objects (for example, storing different
/// handle implementations in a homogeneous collection).
///
/// # Examples
///
/// ```
/// use al_structures::collections::storage::{RwLockStorage, utils::ordered::OrderedDynHandle};
/// use std::collections::hash_map::HashMap;
///
/// let boxed: Box<dyn OrderedDynHandle<String, u8>> = Box::new(RwLockStorage::new(HashMap::new()));
///
/// boxed.insert("x".to_string(), 1).unwrap();
/// assert_eq!(boxed.get(&"x".to_string()).unwrap().unwrap(), 1);
/// ```
pub trait OrderedDynHandle<K, V>: Send + Sync {
    // ---- reads ----
    /// Returns an owned `V` if it exists, use `with_ref` to avoid cloning.
    fn get(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Ord,
        V: Clone;
    /// Returns `Some(R)`, if the key exists, by running `f` on a borrowed `&V`.
    fn with_ref(&self, key: &K, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        K: Ord;
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
        K: Ord;
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
        K: Ord;
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
        K: Ord;
    /// Remove all entries.
    fn clear(&self) -> Result<(), HandleError>;
    /// Retain only entries for which `f` returns `true`.
    fn retain(&self, f: &mut dyn FnMut(&K, &mut V) -> bool) -> Result<(), HandleError>;
    /// Remove all entries and return them as a `Vec`.
    fn drain(&self) -> Result<Vec<(K, V)>, HandleError>;
    /// Insert all entries from an iterator.
    fn extend(&self, iter: &mut dyn Iterator<Item = (K, V)>) -> Result<(), HandleError>;

    /// Merge another `OrderedDynHandle` into this one.
    fn merge(&self, other: &dyn OrderedDynHandle<K, V>) -> Result<(), HandleError>
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

impl<K, V, T: OrderedHandle<K, V> + Send + Sync> OrderedDynHandle<K, V> for T {
    fn get(&self, key: &K) -> Result<Option<V>, HandleError>
    where
        K: Ord,
        V: Clone,
    {
        self.get(key)
    }

    fn with_ref(&self, key: &K, f: &mut dyn FnMut(&V)) -> Result<bool, HandleError>
    where
        K: Ord,
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
        K: Ord,
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
        K: Ord,
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
        K: Ord,
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
/// Read-only storage behavior for backends with ordered keys.
///
/// `OrderedStorageRead` is meant for containers like `BTreeMap` or other
/// sequence-style backends where the key ordering is meaningful for iteration
/// and key comparison.
pub trait OrderedStorageRead<K, V> {
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
        Q: Ord + ?Sized;

    /// Returns whether the storage contains an entry for `key`.
    fn contains_key<Q>(&self, key: &Q) -> Result<bool, StorageError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;

    /// Returns the number of stored entries.
    fn len(&self) -> Result<usize, StorageError>;

    /// Returns whether the storage is empty.
    fn is_empty(&self) -> Result<bool, StorageError> {
        self.len().map(|l| l == 0)
    }

    /// Returns owned entry pairs for all stored values.
    fn entries(&self) -> Result<Vec<(K, V)>, StorageError>
    where
        K: Clone,
        V: Clone;

    /// Returns owned keys for all stored entries.
    fn keys(&self) -> Result<Vec<K>, StorageError>
    where
        K: Clone;

    /// Returns owned values for all stored entries.
    fn values(&self) -> Result<Vec<V>, StorageError>
    where
        V: Clone;

    /// Returns an iterator over borrowed key/value pairs.
    fn iter(&self) -> Self::Iter<'_>;
}

/// Mutable storage behavior for backends with ordered keys.
///
/// `OrderedStorageWrite` extends `OrderedStorageRead` with insertion,
/// replacement, and removal operations.
pub trait OrderedStorageWrite<K, V>: OrderedStorageRead<K, V> + Default {
    /// Insert a key/value pair into the ordered storage.
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
        Q: Ord + ?Sized;

    /// Removes the entry for `key`, returning the owned value if it existed.
    fn remove<Q>(&mut self, key: &Q) -> Result<Option<V>, StorageError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;

    /// Clears all entries from the ordered storage.
    fn clear(&mut self) -> Result<(), StorageError>;

    /// Retains only the entries that satisfy the predicate.
    fn retain<F>(&mut self, f: F) -> Result<(), StorageError>
    where
        F: FnMut(&K, &mut V) -> bool;

    /// Removes and returns all entries as owned pairs.
    fn drain(&mut self) -> Result<Vec<(K, V)>, StorageError>;

    /// Extends the ordered storage with owned entries from `iter`.
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) -> Result<(), StorageError>;

    /// Merge another storage into this one.
    fn merge<R>(&mut self, other: &R) -> Result<(), StorageError>
    where
        R: OrderedStorageRead<K, V>,
        K: Clone,
        V: Clone,
    {
        self.extend(other.entries()?)
    }

    /// Returns an iterator over borrowed mutable entries.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

/// A complete read-write ordered storage interface.
///
/// This trait combines `OrderedStorageRead` and `OrderedStorageWrite`
/// while adding constructors and lazy-initialisation helpers.
pub trait OrderedStorage<K, V>: OrderedStorageWrite<K, V> {
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
