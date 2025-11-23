use std::sync::Arc;

use crate::{Transport, TransportItemRequirements, TransportRequirements};

pub trait TransformFunction<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + Sync> TransformFunction<T> for F {}

pub trait ApplyTransform<T> {
    fn apply(&self, data: T) -> T {
        data
    }
}

#[derive(Debug)]
pub struct NoOp;

impl<T> ApplyTransform<T> for NoOp {}

pub struct TransformFn<T>(Arc<dyn TransformFunction<T>>);

impl<T, F> From<F> for TransformFn<T>
where
    F: TransformFunction<T> + 'static,
{
    fn from(func: F) -> Self {
        TransformFn(Arc::new(func))
    }
}

impl<T> From<Arc<dyn TransformFunction<T>>> for TransformFn<T> {
    fn from(func: Arc<dyn TransformFunction<T>>) -> Self {
        TransformFn(func)
    }
}

impl<T> ApplyTransform<T> for TransformFn<T> {
    fn apply(&self, data: T) -> T {
        (self.0)(data)
    }
}

impl<T> std::fmt::Debug for TransformFn<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TransformFn")
            .field(&"<TransformFunction>")
            .finish()
    }
}

#[derive(Debug)]
pub struct Transform<T, Send: ApplyTransform<T> = NoOp, Recv: ApplyTransform<T> = NoOp> {
    transport: Arc<dyn Transport<T>>,
    transform_send: Send,
    transform_recv: Recv,
}

impl<
        T,
        S: ApplyTransform<T> + TransportRequirements,
        R: ApplyTransform<T> + TransportRequirements,
    > Transform<T, S, R>
{
    pub fn new(transport: Arc<dyn Transport<T>>, transform_send: S, transform_recv: R) -> Self {
        Self {
            transport,
            transform_send,
            transform_recv,
        }
    }
}

impl<
        T: TransportItemRequirements,
        S: ApplyTransform<T> + TransportRequirements,
        R: ApplyTransform<T> + TransportRequirements,
    > Transport<T> for Transform<T, S, R>
{
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        self.transport
            .send_blocking(self.transform_send.apply(data))
    }

    fn recv_blocking(&self) -> Result<T, crate::TransportError> {
        Ok(self.transform_recv.apply(self.transport.recv_blocking()?))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, crate::TransportError> {
        Ok(self
            .transport
            .recv_avaliable_blocking()?
            .into_iter()
            .map(|data| self.transform_recv.apply(data))
            .collect())
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, crate::TransportError> {
        Ok(self
            .transport
            .try_recv_blocking()?
            .and_then(|data| Some(self.transform_recv.apply(data))))
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
        self.transport.send(self.transform_send.apply(data))
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
        Box::pin(async { Ok(self.transform_recv.apply(self.transport.recv().await?)) })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(self
                .transport
                .recv_avaliable()
                .await?
                .into_iter()
                .map(|data| self.transform_recv.apply(data))
                .collect())
        })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(self
                .transport
                .try_recv()
                .await?
                .and_then(|data| Some(self.transform_recv.apply(data))))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event,
        transports::transform::{Transform, TransformFn},
        Queue, Transport,
    };

    #[event]
    struct AddOne(u8);

    #[test]
    fn transform() {
        let transform = Transform::new(
            Queue::new().into(),
            TransformFn::from(|mut x: AddOne| {
                x.0 += 1;
                x
            }),
            TransformFn::from(|mut x: AddOne| {
                x.0 += 1;
                x
            }),
        );

        let x = 1u8;
        transform.send_blocking(AddOne(x)).unwrap();
        let y = transform.recv_blocking().unwrap();
        assert_eq!(y.0, x + 2);
    }
}
