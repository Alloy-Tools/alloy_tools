use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// A generic registry type using a HashMap.
pub type Registry<K, V> = HashMap<K, V>;

/// A thread-safe shared registry using Arc and RwLock.
pub type SharedRegistry<K, V> = Arc<RwLock<Registry<K, V>>>;
