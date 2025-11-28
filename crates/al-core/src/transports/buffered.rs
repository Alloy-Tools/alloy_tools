use crate::{Link, Transport, TransportError, TransportItemRequirements};
use std::sync::Arc;

#[derive(Debug)]
pub struct Buffered<T: TransportItemRequirements>(Link<T>, Arc<dyn Transport<T>>);

impl<T: TransportItemRequirements> Buffered<T> {
    pub fn new(transport: Arc<dyn Transport<T>>) -> Self {
        Self(
            Link::new(crate::Queue::new().into(), transport.clone()),
            transport,
        )
    }
}
/// The `send()` and `send_blocking()` go to the `Queue` within the `Link` while the `recv()` and `recv_blocking()` go to the passed `Transport<T>`
impl<T: TransportItemRequirements> Transport<T> for Buffered<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        self.0.send_blocking(data)
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
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>> + Send + Sync + '_>> {
        self.0.send(data)
    }

    fn recv(&self) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = Result<T, TransportError>> + Send + Sync + '_>> {
        self.1.recv()
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, TransportError>> + Send + Sync + '_>> {
        self.1.recv_avaliable()
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<Box<dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, TransportError>> + Send + Sync + '_>> {
        self.1.try_recv()
    }
}
