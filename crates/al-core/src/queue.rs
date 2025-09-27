use crate::{transport::Transport, TransportError, TransportRequirements};
use std::{collections::VecDeque, sync::Mutex};

/// Queue transport to implement FIFO transport
pub struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Queue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.queue.lock() {
            Ok(queue) => f.debug_struct("Queue").field("queue", &*queue).finish(),
            Err(e) => f.debug_struct("Queue").field("queue", &format!("<lock poisoned>: {}", e.to_string())).finish(),
        }
    }
}

impl<T: TransportRequirements> Queue<T> {
    pub fn new() -> Self {
        Queue::<T> {
            queue: Mutex::new(VecDeque::<T>::new()),
        }
    }
}

/// Impl transport for queue in FIFO order, handling the inner mutex for synchronization
impl<T: TransportRequirements> Transport<T> for Queue<T> {
    fn send(&self, data: T) -> Result<(), TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => Ok(guard.push_back(data)),
            Err(e) => Err(e.into()),
        }
    }

    fn recv(&self) -> Result<T, TransportError> {
        match self.queue.lock() {
            Ok(mut guard) => guard
                .pop_front()
                .ok_or_else(|| TransportError::Custom("No data in Queue".to_string())),
            Err(e) => Err(e.into()),
        }
    }
}
