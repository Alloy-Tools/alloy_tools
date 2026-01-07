use std::sync::Arc;

use crate::{NoOp, Transport, TransportItemRequirements, TransportRequirements};

/// Trait for applying a transformation to data passing through a `Transform` transport
pub trait ApplyTransform<T: TransportItemRequirements> {
    #[inline(always)]
    fn apply(&self, data: T) -> T {
        data
    }
}

/// Allow `NoOp` to be used as `ApplyTransform<T>`
impl<T: TransportItemRequirements> crate::ApplyTransform<T> for NoOp {}
impl<T: TransportItemRequirements> From<NoOp> for TransformFn<T> {
    fn from(_: NoOp) -> Self {
        TransformFn::NoOp
    }
}

/// Trait for a function that can be used to transform data passing through a `Transform` transport
pub trait TransformFunction<T: TransportItemRequirements>: Fn(T) -> T + Send + Sync {}
impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + Sync> TransformFunction<T> for F {}

/// Trait for a struct that can be used to transform data passing through a `Transform` transport
pub trait TransformStruct<T: TransportItemRequirements>:
    ApplyTransform<T> + TransportRequirements
{
}
impl<T: TransportItemRequirements, S: ApplyTransform<T> + TransportRequirements> TransformStruct<T>
    for S
{
}

/// Enum to hold either `NoOp`, a function, or a struct to transform data passing through a `Transform` transport
#[derive(Clone)]
pub enum TransformFn<T: TransportItemRequirements> {
    NoOp,
    Fn(Arc<dyn TransformFunction<T>>),
    Struct(Arc<dyn TransformStruct<T>>),
}

/// Impl `ApplyTransform` for `TransformFn` to allow `.apply()` rather than pattern matching in any `Transform`
impl<T: TransportItemRequirements> ApplyTransform<T> for TransformFn<T> {
    fn apply(&self, data: T) -> T {
        match self {
            TransformFn::NoOp => NoOp.apply(data),
            TransformFn::Fn(func) => func(data),
            TransformFn::Struct(s) => s.apply(data),
        }
    }
}

/// Impl block to allow functions to be converted to a `TransformFn` via `Into`
impl<T: TransportItemRequirements, F> From<F> for TransformFn<T>
where
    F: TransformFunction<T> + 'static,
{
    fn from(func: F) -> Self {
        TransformFn::Fn(Arc::new(func))
    }
}

/// Impl block to allow functions already wrapped in an `Arc` to be converted to a `TransformFn` via `Into`
impl<T: TransportItemRequirements> From<Arc<dyn TransformFunction<T>>> for TransformFn<T> {
    fn from(func: Arc<dyn TransformFunction<T>>) -> Self {
        TransformFn::Fn(func)
    }
}

/// Impl `Debug` for `TransformFn` to allow it to be a `Transport` type
impl<T: TransportItemRequirements> std::fmt::Debug for TransformFn<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformFn::NoOp => f.debug_struct("NoOp").finish(),
            // Debug as a unit struct `TransformFn` since we can't print the inner function
            TransformFn::Fn(_) | TransformFn::Struct(_) => f.debug_struct("TransformFn").finish(),
        }
    }
}

/* ********************
  TransformBuilder
******************** */
/// Builder pattern for `Transform` types, allowing calls to be chained together
#[derive(Debug, Clone)]
pub struct TransformBuilder<T: TransportItemRequirements> {
    transport: Arc<dyn Transport<T>>,
    transform_send: TransformFn<T>,
    transform_recv: TransformFn<T>,
}

impl<T: TransportItemRequirements> TransformBuilder<T> {
    /// Returns a new `TransformBuilder` with the passed `send` and `recv` functionality
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

    /// Helper function to return a new `TransformBuilder` with `NoOp` for both `send` and `recv`
    pub fn no_op(transport: Arc<dyn Transport<T>>) -> Self {
        Self::new(transport, NoOp, NoOp)
    }

    /// Set the `TransformBuilder` `send` to the passed functionality
    pub fn with_send(mut self, send: impl Into<TransformFn<T>>) -> Self {
        self.transform_send = send.into();
        self
    }

    /// Set the `TransformBuilder` `recv` to the passed functionality
    pub fn with_recv(mut self, recv: impl Into<TransformFn<T>>) -> Self {
        self.transform_recv = recv.into();
        self
    }

    /// Build the `Transform` from the `TransformBuilder`
    pub fn build(self) -> Transform<T> {
        Transform {
            transport: self.transport,
            transform_send: self.transform_send,
            transform_recv: self.transform_recv,
        }
    }
}

/* ********************
  Transform
******************** */
/// A `Transform` wraps a `Transport` and applies transformations to the data being sent and received.
#[derive(Debug, Clone)]
pub struct Transform<T: TransportItemRequirements> {
    transport: Arc<dyn Transport<T>>,
    transform_send: TransformFn<T>,
    transform_recv: TransformFn<T>,
}

impl<T: TransportItemRequirements> Transform<T> {
    /// Returns a new `TransformBuilder` to allow `Transform` configuration
    pub fn new(transport: Arc<dyn Transport<T>>) -> TransformBuilder<T> {
        TransformBuilder::no_op(transport)
    }

    /// Returns a new `Transform` with the passed `send` and `recv` configuration
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
            .send_blocking(self.transform_send.apply(data))
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

/// Impl block to allow `Transform` to be converted to `Transport` via `Into`
impl<T: TransportItemRequirements> From<Transform<T>> for Arc<dyn Transport<T>> {
    fn from(value: Transform<T>) -> Self {
        Arc::new(value)
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
        #[derive(Debug)]
        struct TestStruct;
        impl ApplyTransform<u8> for TestStruct {
            fn apply(&self, data: u8) -> u8 {
                // Add one but don't overflow
                data.saturating_add(1)
            }
        }
        impl From<TestStruct> for TransformFn<u8> {
            fn from(value: TestStruct) -> Self {
                TransformFn::Struct(Arc::new(value))
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

    #[tokio::test]
    async fn threaded() {
        let transform = std::sync::Arc::new(Transform::from(
            Queue::<u8>::new().into(),
            |x| x + 1,
            |x| x + 1,
        ));
        let transform_clone = transform.clone();
        let handle = std::thread::spawn(move || {
            let received = transform_clone.recv_blocking().unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        transform.send_blocking(40).unwrap();

        handle.join().unwrap();

        let transform_clone = transform.clone();
        let tokio_handle = tokio::spawn(async move {
            let received = transform_clone.recv().await.unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        transform.send(40).await.unwrap();

        tokio_handle.await.unwrap();
    }
}
