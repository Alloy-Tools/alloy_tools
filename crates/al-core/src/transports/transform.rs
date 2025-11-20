use std::sync::Arc;

use crate::{Transport, TransportItemRequirements};

pub trait TransformFn<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + Sync> TransformFn<T> for F {}

struct Transform<T> {
    transport: Arc<dyn Transport<T>>,
    transform_fn: Arc<dyn TransformFn<T>>,
}

pub struct TransformSend<T>(Transform<T>);

impl<T> std::fmt::Debug for TransformSend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformSend")
            .field("transport", &self.0.transport)
            .field("transform_fn", &"<TransformFn>")
            .finish()
    }
}

impl<T: TransportItemRequirements> From<TransformSend<T>> for Arc<dyn Transport<T>> {
    fn from(transform: TransformSend<T>) -> Self {
        Arc::new(transform)
    }
}

impl<T: TransportItemRequirements> Transport<T> for TransformSend<T> {
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        self.0.transport.send_blocking((self.0.transform_fn)(data))
    }

    fn recv_blocking(&self) -> Result<T, crate::TransportError> {
        self.0.transport.recv_blocking()
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.0.transport.send((self.0.transform_fn)(data))
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.0.transport.recv()
    }
}

pub struct TransformRecv<T>(Transform<T>);

impl<T> std::fmt::Debug for TransformRecv<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformRecv")
            .field("transport", &self.0.transport)
            .field("transform_fn", &"<TransformFn>")
            .finish()
    }
}

impl<T: TransportItemRequirements> From<TransformRecv<T>> for Arc<dyn Transport<T>> {
    fn from(transform: TransformRecv<T>) -> Self {
        Arc::new(transform)
    }
}

impl<T: TransportItemRequirements> Transport<T> for TransformRecv<T> {
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        self.0.transport.send_blocking(data)
    }

    fn recv_blocking(&self) -> Result<T, crate::TransportError> {
        Ok((self.0.transform_fn)(self.0.transport.recv_blocking()?))
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.0.transport.send(data)
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move { Ok((self.0.transform_fn)(self.0.transport.recv().await?)) })
    }
}
