use arc_swap::{ArcSwap, Guard};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, MutexGuard},
};

use super::{
    Registry, RegistryBulkRead, RegistryBulkWrite, RegistryError, RegistryRead, RegistryWrite,
};

impl<K, V> Clone for CowRegistry<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Default for CowRegistry<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: std::fmt::Debug + Eq + Hash + Clone, V: std::fmt::Debug + Clone> std::fmt::Debug
    for CowRegistry<K, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.entries() {
            Ok(entries) => f.debug_map().entries(entries).finish(),
            Err(_) => write!(f, "Registry(<locked poisoned>)"),
        }
    }
}

/// A thread‑safe, reader lock‑free registry backed by an atomically‑swapped
/// `HashMap`.
///
/// Reads never block and are wait‑free.  Writes are serialised by a
/// mutex and perform a full clone of the map (copy‑on‑write).
pub struct CowRegistry<K, V> {
    inner: Arc<Inner<K, V>>,
}

/// Internal representation for `CowRegistry`.
///
/// `data` holds the current `Arc<HashMap<K,V>>` and `write_mutex` serialises writers.
/// Readers use `ArcSwap` to obtain lock‑free access to the map.
struct Inner<K, V> {
    data: ArcSwap<HashMap<K, V>>,
    write_mutex: Mutex<()>,
}

impl<K, V> CowRegistry<K, V> {
    fn load(&self) -> Guard<Arc<HashMap<K, V>>> {
        self.inner.data.load()
    }

    fn copy(&self) -> HashMap<K, V>
    where
        K: Clone,
        V: Clone,
    {
        (**self.load()).clone()
    }

    fn store(&self, data: Arc<HashMap<K, V>>) {
        self.inner.data.store(data);
    }

    fn lock(&self) -> Result<MutexGuard<'_, ()>, RegistryError> {
        self.inner
            .write_mutex
            .lock()
            .map_err(|e| RegistryError::LockPoisoned(e.to_string()))
    }
}

impl<K: Eq + Hash, V: Clone> RegistryRead<K, V> for CowRegistry<K, V> {
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.load().get(key).cloned())
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, RegistryError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Ok(self.load().contains_key(key))
    }

    fn len(&self) -> Result<usize, RegistryError> {
        Ok(self.load().len())
    }

    fn entries(&self) -> Result<Vec<(K, V)>, RegistryError>
    where
        K: Clone,
    {
        Ok(self
            .load()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    fn keys(&self) -> Result<Vec<K>, RegistryError>
    where
        K: Clone,
    {
        Ok(self.load().keys().cloned().collect())
    }

    fn values(&self) -> Result<Vec<V>, RegistryError> {
        Ok(self.load().values().cloned().collect())
    }
}

impl<K: Eq + Hash + Clone, V: Clone> RegistryWrite<K, V> for CowRegistry<K, V> {
    fn insert(&self, key: K, value: V) -> Result<Option<V>, RegistryError> {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.insert(key, value);
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        let old = new_map.remove(key);
        self.store(Arc::new(new_map));
        Ok(old)
    }

    fn clear(&self) -> Result<(), RegistryError> {
        let _guard = self.lock()?;
        self.store(Arc::new(HashMap::new()));
        Ok(())
    }

    fn retain<F>(&self, f: F) -> Result<(), RegistryError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let _guard = self.lock()?;
        let mut new_map = self.copy();
        new_map.retain(f);
        self.store(Arc::new(new_map));
        Ok(())
    }

    fn drain(&self) -> Result<Vec<(K, V)>, RegistryError> {
        let _guard = self.lock()?;
        let drained = self
            .load()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.store(Arc::new(HashMap::new()));
        Ok(drained)
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), RegistryError> {
        let _guard = self.lock();
        let mut new_map = self.copy();
        new_map.extend(iter);
        self.store(Arc::new(new_map));
        Ok(())
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Registry<K, V> for CowRegistry<K, V> {
    fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                data: ArcSwap::from(Arc::new(HashMap::new())),
                write_mutex: Mutex::new(()),
            }),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                data: ArcSwap::from(Arc::new(HashMap::with_capacity(capacity))),
                write_mutex: Mutex::new(()),
            }),
        }
    }

    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            inner: Arc::new(Inner {
                data: ArcSwap::from(Arc::new(HashMap::from_iter(iter))),
                write_mutex: Mutex::new(()),
            }),
        }
    }

    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> V,
    {
        // fast lock‑free check
        if let Some(v) = self.get(&key)? {
            return Ok(v);
        }
        let _guard = self.lock()?;
        // re‑check under lock
        if let Some(v) = self.get(&key)? {
            return Ok(v);
        }
        let value = f();
        let mut new_map = self.copy();
        new_map.insert(key, value.clone());
        self.inner.data.store(Arc::new(new_map));
        Ok(value)
    }

    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> Result<V, RegistryError>,
    {
        // fast lock‑free check
        if let Some(v) = self.get(&key)? {
            return Ok(v);
        }
        let _guard = self.lock()?;
        // re‑check under lock
        if let Some(v) = self.get(&key)? {
            return Ok(v);
        }
        let value = f()?;
        let mut new_map = self.copy();
        new_map.insert(key, value.clone());
        self.inner.data.store(Arc::new(new_map));
        Ok(value)
    }
}

impl<K: Eq + Hash, V: Clone> RegistryBulkRead<K, V, HashMap<K, V>> for CowRegistry<K, V> {
    fn with_read<F, R>(&self, f: F) -> Result<R, super::RegistryError>
    where
        F: FnOnce(&HashMap<K, V>) -> R,
    {
        Ok(f(&self.load()))
    }
}

impl<K: Eq + Hash + Clone, V: Clone> RegistryBulkWrite<K, V, HashMap<K, V>> for CowRegistry<K, V> {
    fn with_write<F, R>(&self, f: F) -> Result<R, super::RegistryError>
    where
        F: FnOnce(&mut HashMap<K, V>) -> R,
    {
        let _guard = self.lock()?;
        let mut new_map = (**self.load()).clone();
        let ret = f(&mut new_map);
        self.store(Arc::new(new_map));
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::collections::registries::{
        RegistryBulkRead, RegistryBulkWrite, RegistryRead, RegistryReadExt, RegistryWrite,
        RegistryWriteExt,
    };

    #[test]
    fn read_write() {
        let registry = CowRegistry::new();
        assert_eq!(registry.len().unwrap(), 0);

        assert!(registry.insert("one".to_string(), 1).unwrap().is_none());
        assert_eq!(registry.get(&"one".to_string()).unwrap(), Some(1));
        assert!(registry.contains_key(&"one".to_string()).unwrap());

        assert_eq!(registry.remove(&"one".to_string()).unwrap(), Some(1));
        assert_eq!(registry.len().unwrap(), 0);
    }

    #[test]
    fn bulk_read_write() {
        let registry = CowRegistry::new();
        registry.insert_unwrap("a".to_string(), 10);

        let got = registry.with_read_unwrap(|map| *map.get("a").unwrap());
        assert_eq!(got, 10);

        let len = registry.with_write_unwrap(|map| {
            map.insert("b".to_string(), 20);
            map.len()
        });
        assert_eq!(len, 2);
        assert_eq!(registry.get_unwrap(&"b".to_string()), Some(20));
    }

    #[test]
    fn snapshots() {
        let registry = CowRegistry::from_iter(vec![
            ("x".to_string(), 1),
            ("y".to_string(), 2),
            ("z".to_string(), 3),
        ]);

        assert_eq!(registry.len_unwrap(), 3);
        assert!(!registry.is_empty_unwrap());

        let mut keys = registry.keys_unwrap();
        keys.sort();
        assert_eq!(
            keys,
            vec!["x".to_string(), "y".to_string(), "z".to_string()]
        );

        let mut values = registry.values_unwrap();
        values.sort();
        assert_eq!(values, vec![1, 2, 3]);

        let mut entries = registry.entries_unwrap();
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
    fn merge_drain_extend_retain_clear() {
        let registry = CowRegistry::from_iter(vec![("a".to_string(), 10)]);
        assert_eq!(registry.len_unwrap(), 1);
        registry.merge_unwrap(&CowRegistry::from_iter(vec![("b".to_string(), 20)]));
        assert_eq!(registry.len_unwrap(), 2);

        let drained = registry.drain().unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(registry.len_unwrap(), 0);

        registry.extend_unwrap(drained);
        assert_eq!(registry.len_unwrap(), 2);
        registry.retain_unwrap(|k, _| k == "a");
        assert_eq!(registry.len_unwrap(), 1);
        registry.clear_unwrap();
        assert_eq!(registry.len_unwrap(), 0);
    }

    #[test]
    fn lazy_initialization() {
        let registry = CowRegistry::new();

        let value = registry
            .get_or_insert_with("x".to_string(), || 100)
            .unwrap();
        assert_eq!(value, 100);
        assert_eq!(registry.get_unwrap(&"x".to_string()), Some(100));

        let value = registry
            .get_or_try_insert_with("y".to_string(), || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
        assert_eq!(registry.get_unwrap(&"y".to_string()), Some(200));
    }

    #[test]
    fn try_lazy_initialization() {
        let registry = CowRegistry::new();

        let value = registry
            .get_or_try_insert_with("y".to_string(), || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
    }

    #[test]
    fn with_capacity() {
        let registry = CowRegistry::with_capacity(100);
        assert_eq!(registry.len_unwrap(), 0);
        registry.insert_unwrap("test".to_string(), 1);
        assert_eq!(registry.len_unwrap(), 1);
    }

    #[test]
    fn from_iter() {
        let registry = CowRegistry::from_iter(vec![("a".to_string(), 1), ("b".to_string(), 2)]);
        assert_eq!(registry.len_unwrap(), 2);
    }

    #[test]
    fn clone() {
        let reg1 = CowRegistry::new();
        reg1.insert_unwrap("x".to_string(), 1);

        let reg2 = reg1.clone();
        assert_eq!(reg2.get_unwrap(&"x".to_string()), Some(1));

        reg2.insert_unwrap("y".to_string(), 2);
        assert_eq!(reg1.get_unwrap(&"y".to_string()), Some(2));
    }
}
