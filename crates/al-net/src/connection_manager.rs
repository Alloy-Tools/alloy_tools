use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

use al_crypto::NonceTrait;
use tokio::sync::RwLock;

use crate::Tcp;

pub struct ConnectionManager<N: NonceTrait> {
    connections: Arc<RwLock<HashMap<u64, Arc<Tcp<N>>>>>,
    next_id: Arc<AtomicU64>,
}

impl<N: NonceTrait> ConnectionManager<N> {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Inserts the connection into the hashmap with the next id, id will loop at u64::MAX back to 0.
    pub async fn insert(&self, tcp: Arc<Tcp<N>>) -> u64 {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.connections.write().await.insert(id, tcp);
        id
    }

    pub async fn get(&self, id: u64) -> Option<Arc<Tcp<N>>> {
        self.connections
            .read()
            .await
            .get(&id)
            .map(|tcp| tcp.clone())
    }
}
