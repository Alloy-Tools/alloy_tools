//! Thread‑safe, extensible registries with a shared interface.
//!
//! # Choosing a registry
//!
//! The two core implementations are [`RwLockRegistry`] and [`CowRegistry`].
//! Both implement the same traits ([`RegistryRead`], [`RegistryWrite`],
//! [`Registry`]) and can be used interchangeably through static dispatch
//! (`impl Registry<K,V>`) or dynamic dispatch ([`DynRegistry`]).
//!
//! | | **RwLockRegistry** | **CowRegistry** |
//! |---|---|---|
//! | **Underlying lock** | `RwLock<HashMap>` | `ArcSwap<HashMap>` + a mutex for writes |
//! | **Read performance** | Acquires a read‑lock (no contention is fast; a waiting writer can block readers) | Lock‑free, wait‑free – always instant |
//! | **Write performance** | O(1) in‑place mutation under a write lock | Clones the entire map on every write (O(n)), serialised by a mutex |
//! | **Memory** | Single live copy of the map | Old copies live until the last reader drops them |
//! | **When to use** | Write‑heavy workloads, large maps, or when you need in‑place mutation | Read‑dominated workloads (configuration, event deserialisers), small–medium maps, or when read latency matters most |
//!
//! For bulk operations that need direct access to the underlying container,
//! both registries implement [`RegistryBulkRead`] and [`RegistryBulkWrite`]
//! with `C = HashMap<K,V>`.
//!
//! To use dynamic dispatch (e.g., storing multiple registry types in a
//! `Vec`), wrap any `Registry<K,V>` with [`DynRegistryBox`] and work through
//! [`DynRegistry`].

mod cow_registry;
mod registry;
mod rwlock_registry;

pub use cow_registry::CowRegistry;
pub use registry::{
    DynRegistry, DynRegistryBox, Registry, RegistryBulkRead, RegistryBulkWrite, RegistryError,
    RegistryRead, RegistryReadExt, RegistryWrite, RegistryWriteExt,
};
pub use rwlock_registry::RwLockRegistry;

#[cfg(test)]
mod dyn_registry {
    mod tests {
        use super::super::*;

        #[test]
        fn read_write() {
            let boxed = DynRegistryBox::new(RwLockRegistry::new());
            let dyn_registry: &dyn DynRegistry<_, _> = &boxed;

            dyn_registry.insert("alpha".to_string(), 42).unwrap();
            assert_eq!(dyn_registry.get(&"alpha".to_string()).unwrap(), Some(42));
            assert!(dyn_registry.contains_key(&"alpha".to_string()).unwrap());
            assert_eq!(dyn_registry.len().unwrap(), 1);
            assert_eq!(dyn_registry.keys().unwrap(), vec!["alpha".to_string()]);
        }

        #[test]
        fn snapshots() {
            let boxed = DynRegistryBox::new(CowRegistry::from_iter(vec![
                ("x".to_string(), 1),
                ("y".to_string(), 2),
            ]));
            let dyn_registry: &dyn DynRegistry<_, _> = &boxed;

            assert_eq!(dyn_registry.len().unwrap(), 2);
            assert!(!dyn_registry.is_empty().unwrap());

            let mut keys = dyn_registry.keys().unwrap();
            keys.sort();
            assert_eq!(keys, vec!["x".to_string(), "y".to_string()]);

            let mut values = dyn_registry.values().unwrap();
            values.sort();
            assert_eq!(values, vec![1, 2]);

            let mut entries = dyn_registry.entries().unwrap();
            entries.sort_by_key(|(k, _)| k.clone());
            assert_eq!(entries, vec![("x".to_string(), 1), ("y".to_string(), 2),]);
        }

        #[test]
        fn merge_drain_extend_retain_clear() {
            let boxed1 = DynRegistryBox::new(RwLockRegistry::new());
            let boxed2 = DynRegistryBox::new(CowRegistry::from_iter(vec![
                ("x".to_string(), 10),
                ("y".to_string(), 20),
            ]));

            let dyn1: &dyn DynRegistry<_, _> = &boxed1;
            let dyn2: &dyn DynRegistry<_, _> = &boxed2;

            dyn1.merge(dyn2).unwrap();
            let drained = dyn1.drain().unwrap();
            assert_eq!(drained.len(), 2);
            assert_eq!(dyn1.len().unwrap(), 0);

            dyn1.extend(&mut drained.into_iter()).unwrap();
            assert_eq!(dyn1.len().unwrap(), 2);
            assert_eq!(dyn1.get(&"x".to_string()).unwrap(), Some(10));
        }

        #[test]
        fn lazy_initialization() {
            let boxed = DynRegistryBox::new(RwLockRegistry::new());
            let dyn_registry: &dyn DynRegistry<_, _> = &boxed;

            let value = dyn_registry
                .get_or_insert_with("key".to_string(), &mut || 100)
                .unwrap();
            assert_eq!(value, 100);
            assert_eq!(dyn_registry.get(&"key".to_string()).unwrap(), Some(100));

            let value = dyn_registry
                .get_or_insert_with("key".to_string(), &mut || 999)
                .unwrap();
            assert_eq!(value, 100);
        }

        #[test]
        fn try_lazy_initialization() {
            let boxed = DynRegistryBox::new(CowRegistry::new());
            let dyn_registry: &dyn DynRegistry<_, _> = &boxed;

            let value = dyn_registry
                .get_or_try_insert_with("key".to_string(), &mut || Ok(200))
                .unwrap();
            assert_eq!(value, 200);
        }

        #[test]
        fn box_into_inner() {
            let registry = RwLockRegistry::new();
            registry.insert("x".to_string(), 42).unwrap();

            let boxed = DynRegistryBox::new(registry);
            let recovered = boxed.into_inner();
            assert_eq!(recovered.get_unwrap(&"x".to_string()), Some(42));
        }

        #[test]
        fn heterogeneous_storage() {
            let registries: Vec<Box<dyn DynRegistry<String, i32>>> = vec![
                Box::new(DynRegistryBox::new(RwLockRegistry::from_iter(vec![(
                    "a".to_string(),
                    1,
                )]))),
                Box::new(DynRegistryBox::new(CowRegistry::from_iter(vec![(
                    "b".to_string(),
                    2,
                )]))),
            ];

            assert_eq!(registries[0].get(&"a".to_string()).unwrap(), Some(1));
            assert_eq!(registries[1].get(&"b".to_string()).unwrap(), Some(2));
            assert_eq!(registries[0].get(&"b".to_string()).unwrap(), None);
        }
    }
}
