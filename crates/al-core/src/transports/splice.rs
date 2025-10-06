use std::{future::Future, marker::PhantomData, sync::Arc};

use crate::{Pipeline, Transport, TransportError, TransportItemRequirements};

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
            .finish()
    }
}

impl<F: TransportItemRequirements, T: TransportItemRequirements> Splice<F, T> {
    /// Returns a new `Splice` joining `producer<F>` into `consumer<T>`
    pub fn new<SpliceFn, Fut>(
        producer: Arc<dyn Transport<F>>,
        consumer: Arc<dyn Transport<T>>,
        splice_fn: Arc<SpliceFn>,
    ) -> Arc<Self>
    where
        SpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, TransportError>> + Send + Sync + 'static,
    {
        let consumer_clone = consumer.clone();
        // Create the new `SpliceTransport<F>` that transforms the data and sends it to the consumer
        let splice_transport = Arc::new(SpliceTransport(
            move |data| {
                let consumer_clone = consumer_clone.clone();
                let splice_fn = splice_fn.clone();
                //TODO: Should this just be started as a task to allow a tight inner loop?
                async move { consumer_clone.send(splice_fn(data).await?).await }
            },
            PhantomData,
        ));

        // Set up a `Link` from the producer to the `SpliceTransport<F>`
        let link = Arc::new(Pipeline::link(producer, splice_transport));

        // setting the new `Link` as the producer in the `Splice`
        Arc::new(Self(link, consumer))
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
        Err(crate::TransportError::NoRecv("Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.".to_string()))
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
}

/* ********************
  SpliceTransport
******************** */

/// `SpliceTransport` acts as a hook to call the `SpliceFn` of the `Splice` that created it
pub struct SpliceTransport<
    F: TransportItemRequirements,
    SpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
>(SpliceFn, PhantomData<(F, Fut)>);

impl<
        F: TransportItemRequirements,
        SpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
    > std::fmt::Debug for SpliceTransport<F, SpliceFn, Fut>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SpliceTransport")
            .field(&"<SpliceFn>")
            .finish()
    }
}

/// On `send`, calls the `SpliceFn` of the `Splice` that created the `SpliceTransport`.
/// Not receivable. Must `recv` from the `Splice` internal consumer.
impl<
        F: TransportItemRequirements,
        SpliceFn: Fn(F) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TransportError>> + Send + Sync + 'static,
    > Transport<F> for SpliceTransport<F, SpliceFn, Fut>
{
    fn send_blocking(&self, _: F) -> Result<(), TransportError> {
        Err(TransportError::NoRecv(
            "Not blockable. Must `send` from the `Splice` intead.".to_string(),
        ))
    }

    fn recv_blocking(&self) -> Result<F, TransportError> {
        Err(TransportError::NoRecv(
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
        Box::pin(async { self.0(data).await })
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
}
