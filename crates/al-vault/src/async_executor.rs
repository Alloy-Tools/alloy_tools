use std::{future::Future, marker::PhantomData, pin::Pin};

pub struct JoinError(pub String);

pub trait AsyncExecutor {
    type Output: Send + 'static;
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
    fn spawn_with_handle(
        &self,
        future: Pin<Box<dyn Future<Output = Self::Output> + Send>>,
    ) -> Box<dyn JoinHandle<Output = Self::Output>>;
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
pub struct TokioExecutor<T>(PhantomData<T>);
impl<T> TokioExecutor<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
#[cfg(feature = "tokio")]
pub struct TokioJoinHandle<T>(tokio::task::JoinHandle<T>);

#[cfg(feature = "tokio")]
impl<T: Send + 'static> AsyncExecutor for TokioExecutor<T> {
    type Output = T;

    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }

    fn spawn_with_handle(
        &self,
        future: Pin<Box<dyn Future<Output = Self::Output> + Send>>,
    ) -> Box<dyn JoinHandle<Output = Self::Output>> {
        Box::new(TokioJoinHandle(tokio::spawn(future)))
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
