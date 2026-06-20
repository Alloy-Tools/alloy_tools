use super::{Registry, RegistryError, RegistryRead};
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

impl<K, V> Clone for RwLockRegistry<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K: Eq + Hash, V: Clone> Default for RwLockRegistry<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: std::fmt::Debug + Eq + Hash + Clone, V: std::fmt::Debug + Clone> std::fmt::Debug
    for RwLockRegistry<K, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.entries() {
            Ok(entries) => f.debug_map().entries(entries).finish(),
            Err(_) => write!(f, "Registry(<locked poisoned>)"),
        }
    }
}

/// A thread‑safe registry backed by an `RwLock<HashMap<K,V>>`.
///
/// This implementation favours write efficiency and memory usage: writes
/// perform in‑place mutation under a write lock while reads acquire a shared
/// read lock. Use `RwLockRegistry` when your workload performs frequent
/// mutations or when keeping a single shared map instance is important.
pub struct RwLockRegistry<K, V> {
    inner: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> RwLockRegistry<K, V> {
    /// Acquire a read lock on the internal map, mapping poisoning errors to `RegistryError`.
    fn read_lock(&self) -> Result<RwLockReadGuard<'_, HashMap<K, V>>, RegistryError> {
        self.inner
            .read()
            .map_err(|e| RegistryError::LockPoisoned(e.to_string()))
    }

    /// Acquire a write lock on the internal map, mapping poisoning errors to `RegistryError`.
    ///
    /// Callers should avoid holding this lock while running long or blocking
    /// operations as it will block concurrent readers.
    fn write_lock(&self) -> Result<RwLockWriteGuard<'_, HashMap<K, V>>, RegistryError> {
        self.inner
            .write()
            .map_err(|e| RegistryError::LockPoisoned(e.to_string()))
    }
}

impl<K: Eq + Hash, V: Clone> RegistryRead<K, V> for RwLockRegistry<K, V> {
    fn get<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.read_lock().map(|map| map.get(key).cloned())
    }

    fn contains_key<Q>(&self, key: &Q) -> Result<bool, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.read_lock().map(|map| map.contains_key(key))
    }

    fn len(&self) -> Result<usize, RegistryError> {
        self.read_lock().map(|map| map.len())
    }

    fn entries(&self) -> Result<Vec<(K, V)>, RegistryError>
    where
        K: Clone,
    {
        self.read_lock()
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    fn keys(&self) -> Result<Vec<K>, RegistryError>
    where
        K: Clone,
    {
        self.read_lock().map(|map| map.keys().cloned().collect())
    }

    fn values(&self) -> Result<Vec<V>, RegistryError> {
        self.read_lock().map(|map| map.values().cloned().collect())
    }
}

impl<K: Eq + Hash, V: Clone> super::RegistryWrite<K, V> for RwLockRegistry<K, V> {
    fn insert(&self, key: K, value: V) -> Result<Option<V>, RegistryError> {
        self.write_lock().map(|mut map| map.insert(key, value))
    }

    fn remove<Q>(&self, key: &Q) -> Result<Option<V>, RegistryError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.write_lock().map(|mut map| map.remove(key))
    }

    fn clear(&self) -> Result<(), RegistryError> {
        self.write_lock().map(|mut map| map.clear())
    }

    fn retain<F>(&self, f: F) -> Result<(), RegistryError>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.write_lock().map(|mut map| map.retain(f))
    }

    fn drain(&self) -> Result<Vec<(K, V)>, RegistryError> {
        self.write_lock().map(|mut map| map.drain().collect())
    }

    fn extend<I: IntoIterator<Item = (K, V)>>(&self, iter: I) -> Result<(), RegistryError> {
        self.write_lock().map(|mut map| map.extend(iter))
    }
}

impl<K: Eq + Hash, V: Clone> Registry<K, V> for RwLockRegistry<K, V> {
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
        }
    }

    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::from_iter(iter))),
        }
    }

    /// Return the value for `key` if it exists; otherwise call `f`,
    /// insert the result, and return a clone.
    ///
    /// The closure is called **while the write lock is held**, so it should
    /// finish quickly.
    fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> V,
    {
        // First try a read lock to avoid writer
        {
            let guard = self.read_lock()?;
            if let Some(value) = guard.get(&key) {
                return Ok(value.clone());
            }
        }

        let mut guard = self.write_lock()?;
        // Double check as another thread may have inserted while waiting
        if let Some(value) = guard.get(&key) {
            return Ok(value.clone());
        }
        let value = f();
        guard.insert(key, value.clone());
        Ok(value)
    }

    /// Fallible version of `get_or_insert_with`.
    /// The closure returns a `Result<V, Err>` with the error being
    /// converted into `RegistryError::InitializationFailed` using `ToString`.
    fn get_or_try_insert_with<F>(&self, key: K, f: F) -> Result<V, RegistryError>
    where
        F: FnOnce() -> Result<V, RegistryError>,
    {
        {
            let guard = self.read_lock()?;
            if let Some(value) = guard.get(&key) {
                return Ok(value.clone());
            }
        }

        let mut guard = self.write_lock()?;
        if let Some(value) = guard.get(&key) {
            return Ok(value.clone());
        }
        let value = f().map_err(|e| RegistryError::InitializationFailed(e.to_string()))?;
        guard.insert(key, value.clone());
        Ok(value)
    }
}

impl<K: Eq + Hash, V: Clone> super::RegistryBulkRead<K, V, HashMap<K, V>> for RwLockRegistry<K, V> {
    fn with_read<F, R>(&self, f: F) -> Result<R, RegistryError>
    where
        F: FnOnce(&HashMap<K, V>) -> R,
    {
        let guard = self
            .inner
            .read()
            .map_err(|e| RegistryError::LockPoisoned(e.to_string()))?;
        Ok(f(&guard))
    }
}

impl<K: Eq + Hash + Clone, V: Clone> super::RegistryBulkWrite<K, V, HashMap<K, V>>
    for RwLockRegistry<K, V>
{
    fn with_write<F, R>(&self, f: F) -> Result<R, RegistryError>
    where
        F: FnOnce(&mut HashMap<K, V>) -> R,
    {
        let mut guard = self
            .inner
            .write()
            .map_err(|e| RegistryError::LockPoisoned(e.to_string()))?;
        Ok(f(&mut guard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::registries::{
        RegistryBulkRead, RegistryBulkWrite, RegistryRead, RegistryReadExt, RegistryWrite,
        RegistryWriteExt,
    };

    #[test]
    fn read_write() {
        let registry = RwLockRegistry::new();
        assert_eq!(registry.len().unwrap(), 0);

        assert!(registry.insert("one".to_string(), 1).unwrap().is_none());
        assert_eq!(registry.get(&"one".to_string()).unwrap(), Some(1));
        assert!(registry.contains_key(&"one".to_string()).unwrap());
        assert_eq!(registry.len().unwrap(), 1);

        assert_eq!(registry.remove(&"one".to_string()).unwrap(), Some(1));
        assert_eq!(registry.len().unwrap(), 0);
    }

    #[test]
    fn bulk_read_write() {
        let registry = RwLockRegistry::new();
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
        let registry = RwLockRegistry::from_iter(vec![
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
                ("z".to_string(), 3)
            ]
        );
    }

    #[test]
    fn merge_drain_extend_retain_clear() {
        let registry = RwLockRegistry::from_iter(vec![("a".to_string(), 10)]);
        assert_eq!(registry.len_unwrap(), 1);
        registry.merge_unwrap(&RwLockRegistry::from_iter(vec![("b".to_string(), 20)]));
        assert_eq!(registry.len_unwrap(), 2);

        let drained = registry.drain_unwrap();
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
        let registry = RwLockRegistry::new();

        let value = registry
            .get_or_insert_with("x".to_string(), || 100)
            .unwrap();
        assert_eq!(value, 100);
        assert_eq!(registry.get_unwrap(&"x".to_string()), Some(100));

        let value = registry
            .get_or_insert_with("x".to_string(), || 999)
            .unwrap();
        assert_eq!(value, 100);
    }

    #[test]
    fn try_lazy_initialization() {
        let registry = RwLockRegistry::new();

        let value = registry
            .get_or_try_insert_with("y".to_string(), || Ok(200))
            .unwrap();
        assert_eq!(value, 200);
    }

    #[test]
    fn with_capacity() {
        let registry = RwLockRegistry::with_capacity(100);
        assert_eq!(registry.len_unwrap(), 0);
        registry.insert_unwrap("test".to_string(), 1);
        assert_eq!(registry.len_unwrap(), 1);
    }

    #[test]
    fn from_iter() {
        let registry = RwLockRegistry::from_iter(vec![("a".to_string(), 1), ("b".to_string(), 2)]);
        assert_eq!(registry.len_unwrap(), 2);
    }

    #[test]
    fn clone() {
        let reg1 = RwLockRegistry::new();
        reg1.insert_unwrap("x".to_string(), 1);

        let reg2 = reg1.clone();
        assert_eq!(reg2.get_unwrap(&"x".to_string()), Some(1));

        reg2.insert_unwrap("y".to_string(), 2);
        assert_eq!(reg1.get_unwrap(&"y".to_string()), Some(2));
    }
}
