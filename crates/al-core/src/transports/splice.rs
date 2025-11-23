use std::{future::Future, marker::PhantomData, sync::Arc};

use crate::{Link, Transport, TransportError, TransportItemRequirements};

pub struct Splice<F: TransportItemRequirements, T: TransportItemRequirements>(
    Arc<dyn Transport<F>>,
    Arc<dyn Transport<T>>,
);

impl<F: TransportItemRequirements, T: TransportItemRequirements> std::fmt::Debug for Splice<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Splice")
            .field(&self.0)
            .field(&self.1)
            .field(&"<SpliceFn>")
            .field(&"<AsyncSpliceFn>")
            .finish()
    }
}

impl<F: TransportItemRequirements, T: TransportItemRequirements> From<Splice<F, T>>
    for Arc<dyn Transport<F>>
{
    fn from(splice: Splice<F, T>) -> Self {
        Arc::new(splice)
    }
}

impl<F: TransportItemRequirements, T: TransportItemRequirements> Splice<F, T> {
    /// Returns a new `Splice` joining `producer<F>` into `consumer<T>`
    pub fn new<SpliceFn, AsyncSpliceFn, Fut>(
        producer: Arc<dyn Transport<F>>,
        consumer: Arc<dyn Transport<T>>,
        splice_fn: Arc<SpliceFn>,
        async_splice_fn: Arc<AsyncSpliceFn>,
    ) -> Arc<Self>
    where
        SpliceFn: Fn(F) -> Result<T, TransportError> + Send + Sync + 'static,
        AsyncSpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, TransportError>> + Send + Sync + 'static,
    {
        let consumer_clone = consumer.clone();
        let async_consumer_clone = consumer.clone();
        // Create the new `SpliceTransport<F>` that transforms the data and sends it to the consumer
        let splice_transport = Arc::new(SpliceTransport(
            move |data| consumer_clone.send_blocking(splice_fn(data)?),
            move |data| {
                let async_consumer_clone = async_consumer_clone.clone();
                let async_splice_fn = async_splice_fn.clone();
                //TODO: Should this just be started as a task to allow a tight inner loop?
                async move {
                    async_consumer_clone
                        .send(async_splice_fn(data).await?)
                        .await
                }
            },
            PhantomData,
        ));

        // Set up a `Link` from the producer to the `SpliceTransport<F>`
        let link = Link::new(producer, splice_transport);

        // setting the new `Link` as the producer in the `Splice`
        Arc::new(Self(link.into(), consumer))
    }

    #[allow(unused)]
    /// Returns the `Splice` internal producer, a `dyn Transport<F>`
    pub fn producer(&self) -> &Arc<dyn Transport<F>> {
        &self.0
    }

    #[allow(unused)]
    /// Returns the `Splice` internal consumer, a `dyn Transport<T>`
    pub fn consumer(&self) -> &Arc<dyn Transport<T>> {
        &self.1
    }
}

/// On `send`, calls `send` on the internal `splice.producer<F>()`.
/// Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.
impl<F: TransportItemRequirements, T: TransportItemRequirements> Transport<F> for Splice<F, T> {
    fn send_blocking(&self, data: F) -> Result<(), crate::TransportError> {
        self.0.send_blocking(data)
    }

    fn recv_blocking(&self) -> Result<F, crate::TransportError> {
        Err(TransportError::UnSupported("Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.".to_string()))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<F>, TransportError> {
        Err(TransportError::UnSupported("Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.".to_string()))
    }

    fn try_recv_blocking(&self) -> Result<Option<F>, TransportError> {
        Err(TransportError::UnSupported("Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.".to_string()))
    }

    fn send(
        &self,
        data: F,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.0.send(data).await })
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<F, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.recv_blocking() })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<F>, TransportError>> + Send + Sync + '_>>
    {
        Box::pin(async { self.recv_avaliable_blocking() })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Option<F>, TransportError>> + Send + Sync + '_>>
    {
        Box::pin(async { self.try_recv_blocking() })
    }
}

/* ********************
  SpliceTransport
******************** */

/// `SpliceTransport` acts as a hook to call the `SpliceFn` or `AsyncSpliceFn` of the `Splice` that created it
pub struct SpliceTransport<
    F: TransportItemRequirements,
    SpliceFn: Fn(F) -> Result<(), TransportError> + Send + Sync + 'static,
    AsyncSpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
>(SpliceFn, AsyncSpliceFn, PhantomData<(F, Fut)>);

impl<
        F: TransportItemRequirements,
        SpliceFn: Fn(F) -> Result<(), TransportError> + Send + Sync + 'static,
        AsyncSpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
    > std::fmt::Debug for SpliceTransport<F, SpliceFn, AsyncSpliceFn, Fut>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SpliceTransport")
            .field(&"<SpliceFn>")
            .field(&"<AsyncSpliceFn>")
            .finish()
    }
}

/// On `send`, calls the `SpliceFn` or `AsyncSpliceFn` of the `Splice` that created the `SpliceTransport`.
/// Not receivable. Must `recv` from the `Splice` internal consumer.
impl<
        F: TransportItemRequirements,
        SpliceFn: Fn(F) -> Result<(), TransportError> + Send + Sync + 'static,
        AsyncSpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
    > Transport<F> for SpliceTransport<F, SpliceFn, AsyncSpliceFn, Fut>
{
    fn send_blocking(&self, data: F) -> Result<(), TransportError> {
        self.0(data)
    }

    fn recv_blocking(&self) -> Result<F, TransportError> {
        Err(TransportError::UnSupported(
            "Not receivable. Must `recv` or `recv_blocking` from the `Splice` internal consumer."
                .to_string(),
        ))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<F>, TransportError> {
        Err(TransportError::UnSupported(
            "Not receivable. Must `recv` or `recv_blocking` from the `Splice` internal consumer."
                .to_string(),
        ))
    }

    fn try_recv_blocking(&self) -> Result<Option<F>, TransportError> {
        Err(TransportError::UnSupported(
            "Not receivable. Must `recv` or `recv_blocking` from the `Splice` internal consumer."
                .to_string(),
        ))
    }

    fn send(
        &self,
        data: F,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.1(data).await })
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<F, TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async { self.recv_blocking() })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<F>, TransportError>> + Send + Sync + '_>>
    {
        Box::pin(async { self.recv_avaliable_blocking() })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Option<F>, TransportError>> + Send + Sync + '_>>
    {
        Box::pin(async { self.try_recv_blocking() })
    }
}
