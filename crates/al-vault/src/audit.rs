use std::{
    collections::VecDeque,
    env,
    fs::File,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Notify;
use xutex::{AsyncMutex, Mutex};

use crate::async_executor::{AsyncExecutor, JoinError, JoinHandle};

pub const AUDIT_LOG_CAPACITY: usize = 1000;
pub static AUDIT_LOG: once_cell::sync::Lazy<Arc<AuditLog>> =
    once_cell::sync::Lazy::new(|| Arc::new(AuditLog::new(AUDIT_LOG_CAPACITY)));

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditError {
    AuditBuffersFull,
    FlushingLogFull,
    IOError(String),
    JoinError(String),
}

impl From<std::io::Error> for AuditError {
    fn from(value: std::io::Error) -> Self {
        AuditError::IOError(value.to_string())
    }
}

impl From<JoinError> for AuditError {
    fn from(value: JoinError) -> Self {
        AuditError::JoinError(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u128, // milliseconds since epoch
    pub operation: String,
    pub secret_tag: String,
}

impl AuditEntry {
    pub fn new(operation: impl Into<String>, tag: impl Into<String>) -> Self {
        AuditEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            operation: operation.into(),
            secret_tag: tag.into(),
        }
    }
}

pub struct AuditLog {
    entries: Mutex<VecDeque<AuditEntry>>,
    flushing: Arc<AsyncMutex<VecDeque<AuditEntry>>>,
    join_handle: Arc<AsyncMutex<Option<Box<dyn JoinHandle<Output = Result<(), AuditError>>>>>>,
    //join_handle: Arc<AsyncMutex<Option<<E as AsyncExecutor>::JoinHandle<Result<(), AuditError>>>>>,
    stop_flush: Arc<AsyncMutex<bool>>,
    notifier: Arc<Notify>,
    capacity: usize,
    threshold: usize,
    target: usize,
}

impl AuditLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity)),
            flushing: Arc::new(AsyncMutex::new(VecDeque::with_capacity(capacity))),
            join_handle: Arc::new(AsyncMutex::new(None)),
            stop_flush: Arc::new(AsyncMutex::new(false)),
            notifier: Arc::new(Notify::new()),
            capacity,
            threshold: (capacity * 2) / 3, // 2/3 of capacity
            target: (capacity * 3) / 4,    // 3/4 of capacity, leave 1/4 in memory
        }
    }

    pub fn log_entry(&self, entry: AuditEntry) -> Result<(), AuditError> {
        if {
            let mut entries = self.entries.lock();
            entries.push_back(entry);
            entries.len() >= self.threshold
        } {
            if let Err(_) = self.flush() {
                Err(AuditError::AuditBuffersFull)?
            }
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), AuditError> {
        let mut flushing = self.flushing.as_sync().lock();
        match self.capacity - flushing.len() {
            0 => Err(AuditError::FlushingLogFull)?,
            cap => {
                let mut entries = self.entries.lock();
                let len = entries.len();
                // Try and take the target, or at most the avaliable entries, or at most the avaliable space
                flushing.extend(entries.drain(..self.target.min(len).min(cap)));
                self.notifier.notify_one();
            }
        }
        Ok(())
    }

    pub async fn start_file_flush(
        &self,
        executor: &dyn AsyncExecutor<Output = Result<(), AuditError>>,
    ) {
        *self.stop_flush.lock().await = false;
        let mut handle = self.join_handle.lock().await;
        if handle.is_some() {
            return;
        }
        let buffer = self.flushing.clone();
        let notifier = self.notifier.clone();
        let stop_flush = self.stop_flush.clone();

        *handle = Some(executor.spawn_with_handle(Box::pin(async move {
            let path = env::current_dir()?;
            println!("current dir: {}", path.display());
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
        })));
    }

    pub async fn stop_file_flush(&self) -> Result<(), AuditError> {
        *self.stop_flush.lock().await = true;
        if let Some(handle) = self.join_handle.lock().await.take() {
            handle.join().await?
        } else {
            Ok(())
        }
    }

    pub async fn abort_file_flush(&self) {
        if let Some(handle) = self.join_handle.lock().await.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{async_executor::TokioExecutor, AUDIT_LOG};

    #[tokio::test]
    async fn todo() {
        AUDIT_LOG.start_file_flush(&TokioExecutor::new()).await
    }
}
