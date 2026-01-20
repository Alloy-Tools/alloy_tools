use std::{collections::VecDeque, fs::File, future::Future, pin::Pin, sync::Arc};
use tokio::sync::Notify;
use xutex::AsyncMutex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditError {
    AuditBuffersFull,
    FlushingLogFull,
    IOError(String),
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u128, // milliseconds since epoch
    pub operation: String,
    pub secret_tag: String,
}

pub struct AuditLog {
    entries: AsyncMutex<VecDeque<AuditEntry>>,
    flushing: Arc<AsyncMutex<VecDeque<AuditEntry>>>,
    join_handle:
        Arc<AsyncMutex<Option<Box<dyn JoinHandle<Output = Result<(), ()>> + Send + 'static>>>>,
    stop_flush: Arc<AsyncMutex<bool>>,
    notifier: Arc<Notify>,
    capacity: usize,
    threshold: usize,
    target: usize,
}

impl AuditLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: AsyncMutex::new(VecDeque::with_capacity(capacity)),
            flushing: Arc::new(AsyncMutex::new(VecDeque::with_capacity(capacity))),
            join_handle: Arc::new(AsyncMutex::new(None)),
            stop_flush: Arc::new(AsyncMutex::new(false)),
            notifier: Arc::new(Notify::new()),
            capacity,
            threshold: (capacity * 2) / 3, // 2/3 of capacity
            target: (capacity * 3) / 4,    // 3/4 of capacity, leave 1/4 in memory
        }
    }

    pub async fn add_entry(&self, entry: AuditEntry) -> Result<(), AuditError> {
        if {
            let mut entries = self.entries.lock().await;
            entries.push_back(entry);
            entries.len() >= self.threshold
        } {
            if let Err(_) = self.flush().await {
                Err(AuditError::AuditBuffersFull)?
            }
        }
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), AuditError> {
        let mut flushing = self.flushing.lock().await;
        match self.capacity - flushing.len() {
            0 => Err(AuditError::FlushingLogFull)?,
            cap => {
                let mut entries = self.entries.lock().await;
                let len = entries.len();
                // Try and take the target, or at most the avaliable entries, or at most the avaliable space
                flushing.extend(entries.drain(..self.target.min(len).min(cap)));
                self.notifier.notify_one();
            }
        }
        Ok(())
    }

    pub async fn start_file_flush<E: AsyncExecutor>(&mut self, executor: E) {
        *self.stop_flush.lock().await = false;
        let mut handle = self.join_handle.lock().await;
        if let Some(_) = *handle {
            return;
        }
        let buffer = self.flushing.clone();
        let notifier = self.notifier.clone();
        let stop_flush = self.stop_flush.clone();

        *handle = Some(executor.spawn_with_handle(async move {
            //TODO: open the file
            loop {
                buffer.lock().await.drain(..).for_each(|entry| {
                    //TODO: write as line to file
                });

                if *stop_flush.lock().await {
                    break Ok(());
                }

                notifier.notified().await;
            }
        }));
    }

    pub async fn stop_file_flush(&self) {
        *self.stop_flush.lock().await = true;
        if let Some(handle) = self.join_handle.lock().await.as_deref() {
            handle.join().await;
        }
    }

    pub async fn abort_file_flush(&self) {
        if let Some(handle) = self.join_handle.lock().await.as_deref() {
            handle.abort();
        }
    }
}

pub trait AsyncExecutor {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static);
    fn spawn_with_handle<F: Future<Output = T> + Send + 'static, T: Send + 'static>(
        &self,
        future: F,
    ) -> Box<dyn JoinHandle<Output = T> + Send + 'static>;
}

pub trait JoinHandle: Send + 'static {
    type Output: Send + 'static;

    /// Wait for task to complete
    fn join(&self) -> Pin<Box<dyn Future<Output = Self::Output> + Send>>;
    /// Cancel the task
    fn abort(&self);
}

//TODO: My `al-core::Task` should impl the `AsyncExecutor` as a `tokio` wrapper

/*
// Implement for tokio::runtime::Handle
impl AsyncExecutor for tokio::runtime::Handle {
    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.block_on(future)
    }
}

// Implement for tokio::runtime::Runtime
impl AsyncExecutor for tokio::runtime::Runtime {
    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.block_on(future)
    }
}*/
