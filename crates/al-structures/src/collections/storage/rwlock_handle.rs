use super::utils::{
    indexed::{
        IndexedHandle, IndexedHandleRead, IndexedHandleWrite, IndexedStorage, IndexedStorageRead,
        IndexedStorageWrite,
    },
    keyed::{
        KeyedHandle, KeyedHandleRead, KeyedHandleWrite, KeyedStorage, KeyedStorageRead,
        KeyedStorageWrite,
    },
    ordered::{
        OrderedHandle, OrderedHandleRead, OrderedHandleWrite, OrderedStorage, OrderedStorageRead,
        OrderedStorageWrite,
    },
    HandleBulkRead, HandleBulkWrite, HandleError, StorageError,
};
use std::{
    borrow::Borrow,
    hash::Hash,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// A thread‑safe storage backed by an `RwLock<Storage>`.
///
/// This implementation favours write efficiency and memory usage: writes
/// perform in‑place mutation under a write lock while reads acquire a shared
/// read lock. Use `RwLockStorage` when your workload performs frequent
/// mutations or when keeping a single shared map instance is important.
pub struct RwLockStorage<Storage> {
    inner: Arc<RwLock<Storage>>,
}

impl<Storage> Clone for RwLockStorage<Storage> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<Storage: Default> Default for RwLockStorage<Storage> {
    fn default() -> Self {
        Self::new(Storage::default())
    }
}

impl<Storage: std::fmt::Debug> std::fmt::Debug for RwLockStorage<Storage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.read() {
            Ok(inner) => write!(f, "RwLockStorage: {:?}", inner),
            Err(e) => write!(f, "RwLockStorage LockPoisoned: {e}"),
        }
    }
}

impl<Storage> HandleBulkRead<Storage> for RwLockStorage<Storage> {
    fn with_read<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&Storage) -> R,
    {
        Ok(f(&*self.read_lock()?))
    }
}

impl<Storage> HandleBulkWrite<Storage> for RwLockStorage<Storage> {
    fn with_write<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&mut Storage) -> R,
    {
        Ok(f(&mut *self.write_lock()?))
    }
}

impl<Storage> RwLockStorage<Storage> {
    pub fn new(storage: Storage) -> Self {
        Self {
            inner: Arc::new(RwLock::new(storage)),
        }
    }

    /// Acquire a read lock on the internal map, mapping poisoning errors to `HandleError`.
    fn read_lock(&self) -> Result<RwLockReadGuard<'_, Storage>, HandleError> {
        self.inner
            .read()
            .map_err(|e| HandleError::LockPoisoned(e.to_string()))
    }

    /// Acquire a write lock on the internal map, mapping poisoning errors to `HandleError`.
    ///
    /// Callers should avoid holding this lock while running long or blocking
    /// operations as it will block concurrent readers.
    fn write_lock(&self) -> Result<RwLockWriteGuard<'_, Storage>, HandleError> {
        self.inner
            .write()
            .map_err(|e| HandleError::LockPoisoned(e.to_string()))
    }
}

// ----- Keyed Storage -----
impl<Storage: KeyedStorageRead<K, V>, K: Hash + Eq, V> KeyedHandleRead<K, V>
    for RwLockStorage<Storage>
{
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone,
    {
        Ok(self.read_lock()?.get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.read_lock()?.get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((&K, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.read_lock()?.iter().for_each(f))
    }

    fn try_for_each<F: FnMut((&K, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.read_lock()?.iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.read_lock()?
            .contains_key(key)
            .map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.read_lock()?.len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.read_lock()?.entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.read_lock()?.keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.read_lock()?.values().map_err(HandleError::Storage)
    }
}

impl<Storage: KeyedStorageWrite<K, V>, K: Hash + Eq, V> KeyedHandleWrite<K, V>
    for RwLockStorage<Storage>
{
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        self.write_lock()?
            .insert(key, value)
            .map_err(HandleError::Storage)
    }

    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        Ok(self.write_lock()?.try_insert(key, value)?.clone())
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        Ok(self.write_lock()?.try_insert_with(key, value)?.clone())
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        Ok(self.write_lock()?.get_mut(key)?.map(|v| f(v)))
    }

    fn for_each_mut<F: FnMut((&K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.write_lock()?.iter_mut().for_each(f))
    }

    fn try_for_each_mut<F: FnMut((&K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.write_lock()?.iter_mut().try_for_each(f))
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.write_lock()?.remove(key).map_err(HandleError::Storage)
    }

    fn clear(&self) -> Result<(), HandleError> {
        self.write_lock()?.clear().map_err(HandleError::Storage)
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.write_lock()?.retain(f).map_err(HandleError::Storage)
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        self.write_lock()?.drain().map_err(HandleError::Storage)
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), HandleError> {
        self.write_lock()?
            .extend(iter)
            .map_err(HandleError::Storage)
    }
}

impl<Storage: KeyedStorage<K, V>, K: Hash + Eq, V> KeyedHandle<K, V> for RwLockStorage<Storage> {
    type Storage = Storage;
    fn new(storage: Storage) -> Self {
        Self::new(storage)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self::new(Storage::with_capacity(capacity))
    }

    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::new(Storage::from_iter(iter))
    }

    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> V,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_insert_with(key, f)?.clone())
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_try_insert_with(key, f)?.clone())
    }
}

// ----- Ordered Storage -----
impl<Storage: OrderedStorageRead<K, V>, K, V> OrderedHandleRead<K, V> for RwLockStorage<Storage> {
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Clone,
    {
        Ok(self.read_lock()?.get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.read_lock()?.get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((&K, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.read_lock()?.iter().for_each(f))
    }

    fn try_for_each<F: FnMut((&K, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.read_lock()?.iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.read_lock()?
            .contains_key(key)
            .map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.read_lock()?.len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.read_lock()?.entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.read_lock()?.keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.read_lock()?.values().map_err(HandleError::Storage)
    }
}

impl<Storage: OrderedStorageWrite<K, V>, K, V> OrderedHandleWrite<K, V> for RwLockStorage<Storage> {
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        self.write_lock()?
            .insert(key, value)
            .map_err(HandleError::Storage)
    }

    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        Ok(self.write_lock()?.try_insert(key, value)?.clone())
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        Ok(self.write_lock()?.try_insert_with(key, value)?.clone())
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        Ok(self.write_lock()?.get_mut(key)?.map(|v| f(v)))
    }

    fn for_each_mut<F: FnMut((&K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.write_lock()?.iter_mut().for_each(f))
    }

    fn try_for_each_mut<F: FnMut((&K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.write_lock()?.iter_mut().try_for_each(f))
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.write_lock()?.remove(key).map_err(HandleError::Storage)
    }

    fn clear(&self) -> Result<(), HandleError> {
        self.write_lock()?.clear().map_err(HandleError::Storage)
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.write_lock()?.retain(f).map_err(HandleError::Storage)
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        self.write_lock()?.drain().map_err(HandleError::Storage)
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), HandleError> {
        self.write_lock()?
            .extend(iter)
            .map_err(HandleError::Storage)
    }
}

impl<Storage: OrderedStorage<K, V>, K, V> OrderedHandle<K, V> for RwLockStorage<Storage> {
    type Storage = Storage;
    fn new(storage: Storage) -> Self {
        Self::new(storage)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self::new(Storage::with_capacity(capacity))
    }

    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::new(Storage::from_iter(iter))
    }

    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> V,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_insert_with(key, f)?.clone())
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_try_insert_with(key, f)?.clone())
    }
}

// ----- Indexed Storage -----
impl<Storage: IndexedStorageRead<V, Key = K>, K, V> IndexedHandleRead<V>
    for RwLockStorage<Storage>
{
    type Key = K;

    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        V: Clone,
    {
        Ok(self.read_lock()?.get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.read_lock()?.get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((Self::Key, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.read_lock()?.iter().for_each(f))
    }

    fn try_for_each<F: FnMut((Self::Key, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.read_lock()?.iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
    {
        self.read_lock()?
            .contains_key(key)
            .map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.read_lock()?.len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.read_lock()?.entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.read_lock()?.keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.read_lock()?.values().map_err(HandleError::Storage)
    }
}

impl<Storage: IndexedStorageWrite<V, Key = K>, K, V> IndexedHandleWrite<V>
    for RwLockStorage<Storage>
{
    fn push(&self, value: V) -> Result<Self::Key, HandleError> {
        self.write_lock()?.push(value).map_err(HandleError::Storage)
    }

    fn push_clone(&self, value: V) -> Result<(Self::Key, V), HandleError>
    where
        V: Clone,
    {
        Ok((self.write_lock()?.push(value.clone())?, value))
    }

    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        self.write_lock()?
            .insert(key, value)
            .map_err(HandleError::Storage)
    }

    fn try_insert(&self, index: Self::Key, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        Ok(self.write_lock()?.try_insert(index, value)?.clone())
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        Ok(self.write_lock()?.try_insert_with(key, value)?.clone())
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<K> + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        Ok(self.write_lock()?.get_mut(key)?.map(|v| f(v)))
    }

    fn for_each_mut<F: FnMut((K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.write_lock()?.iter_mut().for_each(f))
    }

    fn try_for_each_mut<F: FnMut((K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.write_lock()?.iter_mut().try_for_each(f))
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<K> + Eq + ?Sized,
    {
        self.write_lock()?.remove(key).map_err(HandleError::Storage)
    }

    fn clear(&self) -> Result<(), HandleError> {
        self.write_lock()?.clear().map_err(HandleError::Storage)
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut((K, &V)) -> bool,
    {
        self.write_lock()?.retain(f).map_err(HandleError::Storage)
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        self.write_lock()?.drain(..).map_err(HandleError::Storage)
    }

    fn extend<I: IntoIterator<Item = V>>(&self, iter: I) -> Result<Vec<Self::Key>, HandleError> {
        self.write_lock()?
            .extend(iter)
            .map_err(HandleError::Storage)
    }
}

impl<Storage: IndexedStorage<V, Key = K>, K, V> IndexedHandle<V> for RwLockStorage<Storage> {
    type Storage = Storage;
    fn new(storage: Storage) -> Self {
        Self::new(storage)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self::new(Storage::with_capacity(capacity))
    }

    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        Self::new(Storage::from_iter(iter))
    }

    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> V,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_insert_with(key, f)?.clone())
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: do with a read_lock check first
        Ok(self.write_lock()?.get_or_try_insert_with(key, f)?.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::storage::utils::keyed::{
        KeyedHandle, KeyedHandleRead, KeyedHandleReadExt, KeyedHandleWrite, KeyedHandleWriteExt,
    };
    use std::collections::HashMap;

    #[test]
    fn keyed_read_write() {
        let storage = RwLockStorage::new(HashMap::new());
        assert_eq!(storage.len().unwrap(), 0);

        assert!(storage.insert("one".to_string(), 1).unwrap().is_none());
        assert!(storage.contains_key("one").unwrap());
        assert_eq!(storage.get("one").unwrap().unwrap(), 1);
        assert_eq!(storage.len().unwrap(), 1);

        assert_eq!(storage.remove("one").unwrap().unwrap(), 1);
        assert_eq!(storage.len().unwrap(), 0);
    }

    #[test]
    fn keyed_bulk_read_write() {
        let storage = RwLockStorage::new(HashMap::new());
        storage.insert_unwrap("a".to_string(), 10);

        let got = storage.with_read_unwrap(|map| *map.get("a").unwrap());
        assert_eq!(got, 10);

        let len = storage.with_write_unwrap(|map| {
            map.insert("b".to_string(), 20);
            map.len()
        });
        assert_eq!(len, 2);
        assert_eq!(storage.get_unwrap("b").unwrap(), 20);
    }

    #[test]
    fn keyed_snapshots() {
        let storage = RwLockStorage::<HashMap<String, u8>>::from_iter(vec![
            ("x".to_string(), 1),
            ("y".to_string(), 2),
            ("z".to_string(), 3),
        ]);

        assert_eq!(storage.len_unwrap(), 3);
        assert!(!storage.is_empty_unwrap());

        let mut keys = storage.keys_unwrap();
        keys.sort();
        assert_eq!(
            keys,
            vec!["x".to_string(), "y".to_string(), "z".to_string()]
        );

        let mut values = storage.values_unwrap();
        values.sort();
        assert_eq!(values, vec![1, 2, 3]);

        let mut entries = storage.entries_unwrap();
        entries.sort_by_key(|(k, _)| k.clone());
        assert_eq!(
            entries,
            vec![
                ("x".to_string(), 1),
                ("y".to_string(), 2),
                ("z".to_string(), 3)
            ]
        );
    }

    #[test]
    fn keyed_merge_drain_extend_retain_clear() {
        let storage = RwLockStorage::<HashMap<String, u8>>::from_iter(vec![("a".to_string(), 10)]);
        assert_eq!(storage.len_unwrap(), 1);
        storage.merge_unwrap(&RwLockStorage::<HashMap<String, u8>>::from_iter(vec![(
            "b".to_string(),
            20,
        )]));
        assert_eq!(storage.len_unwrap(), 2);

        let drained = storage.drain_unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(storage.len_unwrap(), 0);

        storage.extend_unwrap(drained);
        assert_eq!(storage.len_unwrap(), 2);
        storage.retain_unwrap(|k, _| k == "a");
        assert_eq!(storage.len_unwrap(), 1);
        storage.clear_unwrap();
        assert_eq!(storage.len_unwrap(), 0);
    }

    #[test]
    fn keyed_lazy_initialization() {
        let storage = RwLockStorage::new(HashMap::new());

        assert_eq!(storage.get_or_insert_with("x", || 100).unwrap(), 100);
        assert_eq!(storage.get_unwrap("x").unwrap(), 100);

        assert_eq!(storage.get_or_insert_with("x", || 999).unwrap(), 100);
    }

    #[test]
    fn keyed_try_lazy_initialization() {
        let storage = RwLockStorage::new(HashMap::new());

        assert_eq!(
            storage.get_or_try_insert_with("y", || Ok(200)).unwrap(),
            200
        );
    }

    #[test]
    fn keyed_with_capacity() {
        let storage = RwLockStorage::<HashMap<String, u8>>::with_capacity(100);
        assert_eq!(storage.len_unwrap(), 0);
        storage.insert_unwrap("test".to_string(), 1);
        assert_eq!(storage.len_unwrap(), 1);
    }

    #[test]
    fn keyed_from_iter() {
        let storage = RwLockStorage::<HashMap<String, u8>>::from_iter(vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
        ]);
        assert_eq!(storage.len_unwrap(), 2);
    }

    #[test]
    fn keyed_clone() {
        let storage_1 = RwLockStorage::new(HashMap::new());
        storage_1.insert_unwrap("x".to_string(), 1);

        let storage_2 = storage_1.clone();
        assert_eq!(storage_2.get_unwrap("x").unwrap(), 1);

        storage_2.insert_unwrap("y".to_string(), 2);
        assert_eq!(storage_1.get_unwrap("y").unwrap(), 2);
    }
}
