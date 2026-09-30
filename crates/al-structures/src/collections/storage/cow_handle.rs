use crate::collections::storage::utils::{
    indexed::{
        IndexedHandle, IndexedHandleRead, IndexedHandleWrite, IndexedStorage, IndexedStorageRead,
        IndexedStorageWrite,
    },
    ordered::{
        OrderedHandle, OrderedHandleRead, OrderedHandleWrite, OrderedStorage, OrderedStorageRead,
        OrderedStorageWrite,
    },
    StorageError,
};

use super::utils::{
    keyed::{
        KeyedHandle, KeyedHandleRead, KeyedHandleWrite, KeyedStorage, KeyedStorageRead,
        KeyedStorageWrite,
    },
    HandleBulkRead, HandleBulkWrite, HandleError,
};
use arc_swap::{ArcSwap, Guard};
use std::{
    borrow::Borrow,
    hash::Hash,
    sync::{Arc, Mutex, MutexGuard},
};

/// A thread‑safe, reader lock‑free storage backed by an atomically‑swapped storage.
///
/// Reads never block and are wait‑free.  Writes are serialised by a
/// mutex and perform a full clone of the map (copy‑on‑write).
pub struct CowStorage<Storage> {
    inner: Arc<Inner<Storage>>,
}

impl<Storage> Clone for CowStorage<Storage> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<Storage: Default> Default for CowStorage<Storage> {
    fn default() -> Self {
        Self::new(Storage::default())
    }
}

impl<Storage: std::fmt::Debug> std::fmt::Debug for CowStorage<Storage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CowStorage: {:?}", self.load())
    }
}

impl<Storage> HandleBulkRead<Storage> for CowStorage<Storage> {
    fn with_read<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&Storage) -> R,
    {
        Ok(f(&self.load()))
    }
}

impl<Storage: Clone> HandleBulkWrite<Storage> for CowStorage<Storage> {
    fn with_write<F, R>(&self, f: F) -> Result<R, HandleError>
    where
        F: FnOnce(&mut Storage) -> R,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = f(&mut new_map);
        self.store(Arc::new(new_map));
        Ok(ret)
    }
}

/// Internal representation for `Cowstorage`.
///
/// `data` holds the current `Arc<Storage>` and `write_mutex` serialises writers.
/// Readers use `ArcSwap` to obtain lock‑free access to the map.
struct Inner<Storage> {
    data: ArcSwap<Storage>,
    write_mutex: Mutex<()>,
}

impl<Storage> CowStorage<Storage> {
    pub fn new(storage: Storage) -> Self {
        Self {
            inner: Arc::new(Inner {
                data: ArcSwap::from(Arc::new(storage)),
                write_mutex: Mutex::new(()),
            }),
        }
    }

    fn load(&self) -> Guard<Arc<Storage>> {
        self.inner.data.load()
    }

    fn copy(&self) -> Storage
    where
        Storage: Clone,
    {
        (**self.load()).clone()
    }

    fn store(&self, data: Arc<Storage>) {
        self.inner.data.store(data);
    }

    fn lock(&self) -> Result<MutexGuard<'_, ()>, HandleError> {
        self.inner
            .write_mutex
            .lock()
            .map_err(|e| HandleError::LockPoisoned(e.to_string()))
    }
}

// ----- Keyed Storage -----
impl<Storage: KeyedStorageRead<K, V>, K: Hash + Eq, V> KeyedHandleRead<K, V>
    for CowStorage<Storage>
{
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone,
    {
        Ok(self.load().get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.load().get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((&K, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.load().iter().for_each(f))
    }

    fn try_for_each<F: FnMut((&K, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.load().iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.load().contains_key(key).map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.load().len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.load().entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.load().keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.load().values().map_err(HandleError::Storage)
    }
}

impl<Storage: KeyedStorageWrite<K, V> + Clone, K: Hash + Eq, V> KeyedHandleWrite<K, V>
    for CowStorage<Storage>
{
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.insert(key, value)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert(key, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert_with(key, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.get_mut(key)?.map(|v| f(v));
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn for_each_mut<F: FnMut((&K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.iter_mut().for_each(f);
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn try_for_each_mut<F: FnMut((&K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.iter_mut().try_for_each(f);
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.remove(key)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn clear(&self) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        self.store(Arc::new(Default::default()));
        Ok(())
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.retain(f)?;
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let drained = new_map.drain()?;
        self.store(Arc::new(new_map));
        Ok(drained)
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.extend(iter)?;
        self.store(Arc::new(new_map));
        Ok(())
    }
}

impl<Storage: KeyedStorage<K, V> + Clone, K: Hash + Eq, V: Clone> KeyedHandle<K, V>
    for CowStorage<Storage>
{
    type Storage = Storage;

    fn new(storage: Self::Storage) -> Self {
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
        //REVIEW: redo with read first
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: redo with read first
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_try_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }
}

// ----- Ordered Storage -----
impl<Storage: OrderedStorageRead<K, V>, K, V> OrderedHandleRead<K, V> for CowStorage<Storage> {
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Clone,
    {
        Ok(self.load().get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.load().get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((&K, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.load().iter().for_each(f))
    }

    fn try_for_each<F: FnMut((&K, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.load().iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.load().contains_key(key).map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.load().len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.load().entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.load().keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.load().values().map_err(HandleError::Storage)
    }
}

impl<Storage: OrderedStorageWrite<K, V> + Clone, K, V> OrderedHandleWrite<K, V>
    for CowStorage<Storage>
{
    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.insert(key, value)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn try_insert(&self, key: K, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert(key, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert_with(key, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.get_mut(key)?.map(|v| f(v));
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn for_each_mut<F: FnMut((&K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.iter_mut().for_each(f);
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn try_for_each_mut<F: FnMut((&K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.iter_mut().try_for_each(f);
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.remove(key)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn clear(&self) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        self.store(Arc::new(Default::default()));
        Ok(())
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.retain(f)?;
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let drained = new_map.drain()?;
        self.store(Arc::new(new_map));
        Ok(drained)
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.extend(iter)?;
        self.store(Arc::new(new_map));
        Ok(())
    }
}

impl<Storage: OrderedStorage<K, V> + Clone, K, V> OrderedHandle<K, V> for CowStorage<Storage> {
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
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: do with a read_lock check first
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_try_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }
}

// ----- Indexed Storage -----
impl<Storage: IndexedStorageRead<V, Key = K>, K, V> IndexedHandleRead<V> for CowStorage<Storage> {
    type Key = K;

    fn get<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        V: Clone,
    {
        Ok(self.load().get(key)?.cloned())
    }

    fn with_ref<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        Ok(self.load().get(key)?.map(|v| f(v)))
    }

    fn for_each<F: FnMut((Self::Key, &V))>(&self, f: F) -> Result<(), HandleError> {
        Ok(self.load().iter().for_each(f))
    }

    fn try_for_each<F: FnMut((Self::Key, &V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        Ok(self.load().iter().try_for_each(f))
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, HandleError>
    where
        Q: Borrow<Self::Key> + Eq + ?Sized,
    {
        self.load().contains_key(key).map_err(HandleError::Storage)
    }

    fn len(&self) -> Result<usize, HandleError> {
        self.load().len().map_err(HandleError::Storage)
    }

    fn entries(&self) -> Result<Vec<(K, V)>, HandleError>
    where
        K: Clone,
        V: Clone,
    {
        self.load().entries().map_err(HandleError::Storage)
    }

    fn keys(&self) -> Result<Vec<K>, HandleError>
    where
        K: Clone,
    {
        self.load().keys().map_err(HandleError::Storage)
    }

    fn values(&self) -> Result<Vec<V>, HandleError>
    where
        V: Clone,
    {
        self.load().values().map_err(HandleError::Storage)
    }
}

impl<Storage: IndexedStorageWrite<V, Key = K> + Clone, K, V> IndexedHandleWrite<V>
    for CowStorage<Storage>
{
    fn push(&self, value: V) -> Result<Self::Key, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let key = new_map.push(value)?;
        self.store(Arc::new(new_map));
        Ok(key)
    }

    fn push_clone(&self, value: V) -> Result<(Self::Key, V), HandleError>
    where
        V: Clone,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let key = new_map.push(value.clone())?;
        self.store(Arc::new(new_map));
        Ok((key, value))
    }

    fn insert(&self, key: K, value: V) -> Result<Option<V>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.insert(key, value)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn try_insert(&self, index: Self::Key, value: V) -> Result<V, HandleError>
    where
        V: Clone,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert(index, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn try_insert_with<F>(&self, key: K, value: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.try_insert_with(key, value)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn with_mut<Q, F, R>(&self, key: &Q, f: F) -> Result<Option<R>, HandleError>
    where
        Q: Borrow<K> + Eq + ?Sized,
        F: FnOnce(&mut V) -> R,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.get_mut(key)?.map(|v| f(v));
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn for_each_mut<F: FnMut((K, &mut V))>(&self, f: F) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.iter_mut().for_each(f);
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn try_for_each_mut<F: FnMut((K, &mut V)) -> std::ops::ControlFlow<B>, B>(
        &self,
        f: F,
    ) -> Result<std::ops::ControlFlow<B>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let ret = new_map.iter_mut().try_for_each(f);
        self.store(Arc::new(new_map));
        Ok(ret)
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, HandleError>
    where
        Q: Borrow<K> + Eq + ?Sized,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.remove(key)?;
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn clear(&self) -> Result<(), HandleError> {
        let _guard = self.lock()?;
        self.store(Arc::new(Default::default()));
        Ok(())
    }

    fn retain<F>(&self, f: F) -> Result<(), HandleError>
    where
        F: FnMut((K, &V)) -> bool,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.retain(f)?;
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn drain(&self) -> Result<Vec<(K, V)>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let drained = new_map.drain(..)?;
        self.store(Arc::new(new_map));
        Ok(drained)
    }

    fn extend<I: IntoIterator<Item = V>>(&self, iter: I) -> Result<Vec<Self::Key>, HandleError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let keys = new_map.extend(iter)?;
        self.store(Arc::new(new_map));
        Ok(keys)
    }
}

impl<Storage: IndexedStorage<V, Key = K> + Clone, K, V> IndexedHandle<V> for CowStorage<Storage> {
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
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, HandleError>
    where
        V: Clone,
        F: FnOnce() -> Result<V, StorageError>,
    {
        //REVIEW: do with a read_lock check first
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let val = new_map.get_or_try_insert_with(key, f)?.clone();
        self.store(Arc::new(new_map));
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::storage::utils::keyed::{
        KeyedHandle, KeyedHandleRead, KeyedHandleReadExt, KeyedHandleWrite, KeyedHandleWriteExt,
    };
    use std::{collections::HashMap, vec};

    #[test]
    fn keyed_read_write() {
        let storage = CowStorage::new(HashMap::new());
        assert_eq!(storage.len().unwrap(), 0);

        assert!(storage.insert("one".to_string(), 1).unwrap().is_none());
        assert_eq!(storage.get("one").unwrap().unwrap(), 1);
        assert!(storage.contains_key("one").unwrap());

        assert_eq!(storage.remove("one").unwrap().unwrap(), 1);
        assert_eq!(storage.len().unwrap(), 0);
    }

    #[test]
    fn keyed_bulk_read_write() {
        let storage = CowStorage::new(HashMap::new());
        storage.insert_unwrap("a".to_string(), 10);

        let got = storage.with_read_unwrap(|map| *map.get("a").unwrap());
        assert_eq!(got, 10);

        let len = storage.with_write_unwrap(|map| {
            map.insert("b".to_string(), 20);
            map.len()
        });
        assert_eq!(len, 2);
        assert_eq!(storage.get("b").unwrap().unwrap(), 20);
    }

    #[test]
    fn keyed_snapshots() {
        let storage = CowStorage::<HashMap<_, _>>::from_iter(vec![
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
                ("z".to_string(), 3),
            ]
        );
    }

    #[test]
    fn keyed_merge_drain_extend_retain_clear() {
        let storage = CowStorage::<HashMap<_, _>>::from_iter(vec![("a".to_string(), 10)]);
        assert_eq!(storage.len_unwrap(), 1);
        storage.merge_unwrap(&CowStorage::<HashMap<_, _>>::from_iter(vec![(
            "b".to_string(),
            20,
        )]));
        assert_eq!(storage.len_unwrap(), 2);

        let drained = storage.drain().unwrap();
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
        let storage = CowStorage::new(HashMap::<String, _>::new());

        let value = storage.get_or_insert_with("x".to_string(), || 100).unwrap();
        assert_eq!(value, 100);
        assert_eq!(storage.get_unwrap("x").unwrap(), 100);

        let value = storage
            .get_or_try_insert_with("y".to_string(), || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
        assert_eq!(storage.get_unwrap("y").unwrap(), 200);
    }

    #[test]
    fn keyed_try_lazy_initialization() {
        let storage = CowStorage::new(HashMap::<String, _>::new());

        let value = storage
            .get_or_try_insert_with("y".to_string(), || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
    }

    #[test]
    fn keyed_with_capacity() {
        let storage = CowStorage::<HashMap<_, _>>::with_capacity(100);
        assert_eq!(storage.len_unwrap(), 0);
        storage.insert_unwrap("test".to_string(), 1);
        assert_eq!(storage.len_unwrap(), 1);
    }

    #[test]
    fn keyed_from_iter() {
        let storage = CowStorage::<HashMap<_, _>>::from_iter(vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
        ]);
        assert_eq!(storage.len_unwrap(), 2);
    }

    #[test]
    fn keyed_clone() {
        let storage_1 = CowStorage::new(HashMap::new());
        storage_1.insert_unwrap("x".to_string(), 1);

        let storage_2 = storage_1.clone();
        assert_eq!(storage_2.get_unwrap("x").unwrap(), 1);

        storage_2.insert_unwrap("y".to_string(), 2);
        assert_eq!(storage_1.get_unwrap("y").unwrap(), 2);
    }
}
