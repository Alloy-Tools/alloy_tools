use std::sync::Arc;

use crate::{NoOp, Transport, TransportItemRequirements, TransportRequirements};

pub trait TransformFunction<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + Sync> TransformFunction<T> for F {}

pub trait ApplyTransform<T> {
    #[inline(always)]
    fn apply(&self, data: T) -> T {
        data
    }
}

/// Allow NoOp to be used as `ApplyTransform<T>` type state
impl<T> crate::ApplyTransform<T> for NoOp {}
impl<T> From<NoOp> for TransformFn<T> {
    fn from(value: NoOp) -> Self {
        TransformFn::NoOp(value)
    }
}

trait TransformStruct<T>: ApplyTransform<T> + TransportRequirements {}
impl<T: ApplyTransform<T> + TransportRequirements> TransformStruct<T> for T {}

#[derive(Clone)]
pub enum TransformFn<T> {
    NoOp(NoOp),
    Fn(Arc<dyn TransformFunction<T>>),
    Struct(Arc<dyn TransformStruct<T>>),
}

impl<T, S> From<S> for TransformFn<T>
where
    S: TransformStruct<T> + 'static,
{
    fn from(value: S) -> Self {
        TransformFn::Struct(Arc::new(value))
    }
}

impl<T, F> From<F> for TransformFn<T>
where
    F: TransformFunction<T> + 'static,
{
    fn from(func: F) -> Self {
        TransformFn::Fn(Arc::new(func))
    }
}

impl<T> From<Arc<dyn TransformFunction<T>>> for TransformFn<T> {
    fn from(func: Arc<dyn TransformFunction<T>>) -> Self {
        TransformFn::Fn(func)
    }
}

impl<T> std::fmt::Debug for TransformFn<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformFn::NoOp(_) => f.debug_struct("NoOp").finish(),
            // Debug as a unit struct `TransformFn` since we can't print the inner function
            TransformFn::Fn(_) | TransformFn::Struct(_) => f.debug_struct("TransformFn").finish(),
        }
    }
}

/*0impl<T> ApplyTransform<T> for TransformFn<T> {
    fn apply(&self, data: T) -> T {
        match self {
            TransformFn::NoOp(no_op) => no_op.apply(data),
            TransformFn::Fn(func) => func(data),
            TransformFn::Struct(s) => s.apply(data),
        }
    }
}*/

impl<T: TransportItemRequirements> From<Transform<T>> for Arc<dyn Transport<T>> {
    fn from(value: Transform<T>) -> Self {
        Arc::new(value)
    }
}

#[derive(Debug, Clone)]
pub struct TransformBuilder<T: TransportItemRequirements> {
    transport: Arc<dyn Transport<T>>,
    transform_send: TransformFn<T>,
    transform_recv: TransformFn<T>,
}

impl<T: TransportItemRequirements> TransformBuilder<T> {
    pub fn new(
        transport: Arc<dyn Transport<T>>,
        send: impl Into<TransformFn<T>>,
        recv: impl Into<TransformFn<T>>,
    ) -> Self {
        TransformBuilder {
            transport,
            transform_send: send.into(),
            transform_recv: recv.into(),
        }
    }

    pub fn with_send(self, send: impl Into<TransformFn<T>>) -> TransformBuilder<T> {
        TransformBuilder::new(self.transport, send.into(), self.transform_recv)
    }

    pub fn with_recv(self, recv: impl Into<TransformFn<T>>) -> TransformBuilder<T> {
        TransformBuilder::new(self.transport, self.transform_send, recv.into())
    }

    pub fn build(self) -> Transform<T> {
        Transform {
            transport: self.transport,
            transform_send: self.transform_send,
            transform_recv: self.transform_recv,
        }
    }
}

impl<T: TransportItemRequirements> TransformBuilder<T> {
    pub fn no_op(transport: Arc<dyn Transport<T>>) -> Self {
        Self::new(transport, NoOp, NoOp)
    }
}

#[derive(Debug, Clone)]
pub struct Transform<T: TransportItemRequirements> {
    transport: Arc<dyn Transport<T>>,
    transform_send: TransformFn<T>,
    transform_recv: TransformFn<T>,
}

impl<T: TransportItemRequirements> Transform<T> {
    pub fn new(transport: Arc<dyn Transport<T>>) -> TransformBuilder<T> {
        TransformBuilder::no_op(transport)
    }

    pub fn from(
        transport: Arc<dyn Transport<T>>,
        send: impl Into<TransformFn<T>>,
        recv: impl Into<TransformFn<T>>,
    ) -> Transform<T> {
        Self::new(transport).with_send(send).with_recv(recv).build()
    }
}

impl<T: TransportItemRequirements> Transport<T> for Transform<T> {
    fn send_blocking(&self, data: T) -> Result<(), crate::TransportError> {
        self.transport
            .send_blocking(match &self.transform_send {
                TransformFn::NoOp(no_op) => no_op.apply(data),
                TransformFn::Fn(func) => func(data),
                TransformFn::Struct(s) => s.apply(data),
            })
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), crate::TransportError> {
        self.transport.send_batch_blocking(
            data.into_iter()
                .map(|d| self.transform_send.apply(d))
                .collect(),
        )
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

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), crate::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.transport.send_batch(
            data.into_iter()
                .map(|d| self.transform_send.apply(d))
                .collect(),
        )
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
    use std::{sync::Arc, vec};

    #[cfg(feature = "event")]
    use crate::event;

    use crate::{
        transports::transform::Transform, ApplyTransform, NoOp, Queue, TransformFn, Transport,
    };

    #[cfg(feature = "event")]
    #[event]
    struct AddOne(u8);

    #[cfg(feature = "event")]
    #[test]
    fn with_event() {
        let transform = Transform::<_>::new(Queue::new().into())
            .with_send(|mut x: AddOne| {
                x.0 += 1;
                x
            })
            .with_recv(|mut x: AddOne| {
                x.0 += 1;
                x
            })
            .build();
        let x = 1u8;
        transform.send_blocking(AddOne(x)).unwrap();
        let y = transform.recv_blocking().unwrap();
        assert_eq!(y.0, x + 2);
    }

    #[test]
    fn with_struct() {
        struct TestStruct;
        impl ApplyTransform<u8> for TestStruct {
            fn apply(&self, data: u8) -> u8 {
                // Add one but don't overflow
                data.saturating_add(1)
            }
        }
        impl From<TestStruct> for TransformFn<u8> {
            fn from(value: TestStruct) -> Self {
                TransformFn::Struct(Arc::new(value.into()))
            }
        }

        let transform = Transform::<u8>::new(Queue::new().into())
            .with_send(TestStruct)
            .build();

        assert_eq!(format!("{:?}", transform), "Transform { transport: Queue { queue: [] }, transform_send: TransformFn, transform_recv: NoOp }")
    }

    #[test]
    fn transform() {
        let transform = Transform::<_>::from(
            Queue::new().into(),
            |mut x| {
                x += 1;
                x
            },
            |mut x| {
                x += 1;
                x
            },
        );

        let x = 1u8;
        transform.send_blocking(x).unwrap();
        let y = transform.recv_blocking().unwrap();
        assert_eq!(y, x + 2);
    }

    #[tokio::test]
    async fn debug() {
        assert_eq!(
            format!("{:?}", Transform::<u8>::from(
                Queue::new().into(),
                NoOp,
                |mut x| {
                    x += 1;
                    x
                }
            )),
            "Transform { transport: Queue { queue: [] }, transform_send: NoOp, transform_recv: TransformFn }"
        );
        assert_eq!(
            format!("{:?}", Transform::<u8>::new(Queue::new().into())
            .with_recv(|mut x| {
                    x += 1;
                    x
                }).build()),
            "Transform { transport: Queue { queue: [] }, transform_send: NoOp, transform_recv: TransformFn }"
        );
    }
    #[tokio::test]
    async fn send_recv() {
        let transform = Transform::<_>::from(
            Queue::new().into(),
            |mut x| {
                x += 1;
                x
            },
            |mut x| {
                x += 1;
                x
            },
        );

        let noop_transform = Transform::<_>::from(Queue::new().into(), NoOp, NoOp);

        let x = 1u8;
        // Send `AddOne` asynchronously
        transform.send(x).await.unwrap();
        transform.send_batch(vec![x + 1, x + 2]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(transform.recv().await.unwrap(), x + 2);
        // Try recv `String` asynchronously
        assert_eq!(transform.try_recv().await.unwrap().unwrap(), x + 3);
        // Recv avaliable `String` asynchronously
        assert_eq!(transform.recv_avaliable().await.unwrap(), vec![x + 4]);

        // Send `AddOne` synchronously
        transform.send_blocking(x).unwrap();
        transform.send_batch_blocking(vec![x + 1, x + 2]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(transform.recv_blocking().unwrap(), x + 2);
        // Try recv `String` synchronously
        assert_eq!(transform.try_recv_blocking().unwrap().unwrap(), x + 3);
        // Recv avaliable `String` synchronously
        assert_eq!(transform.recv_avaliable_blocking().unwrap(), vec![x + 4]);

        let z = 3u8;
        // Send `AddOne` asynchronously
        noop_transform.send(z).await.unwrap();
        noop_transform.send_batch(vec![z + 1, z + 2]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(noop_transform.recv().await.unwrap(), z);
        // Try recv `String` asynchronously
        assert_eq!(noop_transform.try_recv().await.unwrap().unwrap(), z + 1);
        // Recv avaliable `String` asynchronously
        assert_eq!(noop_transform.recv_avaliable().await.unwrap(), vec![z + 2]);

        // Send `AddOne` synchronously
        noop_transform.send_blocking(z).unwrap();
        noop_transform
            .send_batch_blocking(vec![z + 1, z + 2])
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(noop_transform.recv_blocking().unwrap(), z);
        // Try recv `String` synchronously
        assert_eq!(noop_transform.try_recv_blocking().unwrap().unwrap(), z + 1);
        // Recv avaliable `String` synchronously
        assert_eq!(
            noop_transform.recv_avaliable_blocking().unwrap(),
            vec![z + 2]
        );
    }
}
