use std::{future::Future, pin::Pin};
//TODO: Move file to `al-core` for reusability

pub struct JoinError(pub String);

pub trait AsyncExecutor: Send + Sync {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
    fn spawn_with_handle<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        future: F,
    ) -> Box<dyn JoinHandle<Output = T>>;

    fn block_on<T: Send + 'static, F: Future<Output = T> + Send>(&self, future: F) -> T;
}

pub trait JoinHandle: Send + 'static {
    type Output: Send + 'static;

    /// Wait for task to complete
    fn join(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, JoinError>> + Send>>;
    /// Cancel the task
    fn abort(self: Box<Self>);
}

#[cfg(feature = "tokio")]
pub struct TokioExecutor;
#[cfg(feature = "tokio")]
pub struct TokioJoinHandle<T>(tokio::task::JoinHandle<T>);

#[cfg(feature = "tokio")]
impl AsyncExecutor for TokioExecutor {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }

    fn spawn_with_handle<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        future: F,
    ) -> Box<dyn JoinHandle<Output = T>> {
        Box::new(TokioJoinHandle(tokio::spawn(future)))
    }

    fn block_on<T: Send + 'static, F: Future<Output = T> + Send>(&self, future: F) -> T {
        tokio::runtime::Handle::current().block_on(future)
    }
}

#[cfg(feature = "tokio")]
impl<T: Send + 'static> JoinHandle for TokioJoinHandle<T> {
    type Output = T;

    fn join(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, JoinError>> + Send>> {
        Box::pin(async move { self.0.await.map_err(|e| JoinError(e.to_string())) })
    }

    fn abort(self: Box<Self>) {
        self.0.abort();
    }
}
