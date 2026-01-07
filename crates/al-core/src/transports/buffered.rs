use crate::{Link, Transport, TransportError, TransportItemRequirements};
use std::sync::Arc;

pub struct Buffered<T: TransportItemRequirements>(Link<T>, Arc<dyn Transport<T>>);

impl<T: TransportItemRequirements> std::fmt::Debug for Buffered<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Buffered")
            .field(&self.0.producer())
            .field(&self.1)
            .finish()
    }
}

impl<T: TransportItemRequirements> Buffered<T> {
    pub fn new(transport: Arc<dyn Transport<T>>) -> Self {
        Self(
            Link::new(crate::Queue::new().into(), transport.clone()),
            transport,
        )
    }
}

impl<T: TransportItemRequirements> From<Buffered<T>> for Arc<dyn Transport<T>> {
    fn from(value: Buffered<T>) -> Self {
        Arc::new(value)
    }
}

impl<T: TransportItemRequirements> From<Arc<dyn Transport<T>>> for Buffered<T> {
    fn from(value: Arc<dyn Transport<T>>) -> Self {
        Self::new(value)
    }
}

/// The `send()` and `send_blocking()` go to the `Queue` within the `Link` while the `recv()` and `recv_blocking()` go to the passed `Transport<T>`
impl<T: TransportItemRequirements> Transport<T> for Buffered<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        self.0.send_blocking(data)
    }

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        self.0.send_batch_blocking(data)
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        self.1.recv_blocking()
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        self.1.recv_avaliable_blocking()
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        self.1.try_recv_blocking()
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.0.send(data)
    }

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.0.send_batch(data)
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.1.recv()
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.1.recv_avaliable()
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.1.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Buffered, Queue, Transport};

    #[tokio::test]
    async fn debug() {
        assert_eq!(
            format!("{:?}", Buffered::new(Queue::<String>::new().into())),
            "Buffered(Queue { queue: [] }, Queue { queue: [] })"
        );
    }

    #[tokio::test]
    async fn send_recv() {
        let buffered = Buffered::new(Queue::<u8>::new().into());
        buffered.send(1).await.unwrap();
        buffered.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(buffered.recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(buffered.try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(buffered.recv_avaliable().await.unwrap(), vec![3]);

        buffered.send_blocking(1).unwrap();
        buffered.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(buffered.recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(buffered.try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(buffered.recv_avaliable_blocking().unwrap(), vec![3]);
    }

    #[tokio::test]
    async fn threaded() {
        let buffered = std::sync::Arc::new(Buffered::new(Queue::<u8>::new().into()));
        let buffered_clone = buffered.clone();
        let handle = tokio::spawn(async move {
            let received = buffered_clone.recv().await.unwrap();
            assert_eq!(received, 42);
        });

        // Wait to ensure the other thread is receiving the data
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        buffered.send(42).await.unwrap();

        handle.await.unwrap();
    }
}
