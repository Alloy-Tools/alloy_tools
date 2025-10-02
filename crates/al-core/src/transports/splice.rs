use std::{sync::Arc};

use crate::{task::WithTaskState, Pipeline, Task, Transport, TransportError, TransportItemRequirements};

type SpliceFn<F, T> = Arc<dyn Fn(F) -> Result<T, TransportError> + Send + Sync>;

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
    pub fn new(
        producer: Arc<dyn Transport<F>>,
        consumer: Arc<dyn Transport<T>>,
        splice_fn: SpliceFn<F, T>,
    ) -> Arc<Self> {
        // Create the new `SpliceTransport<F>` that transforms the data and sends it to the consumer
        let consumer_clone = consumer.clone();
        let splice_transport = Arc::new(SpliceTransport::<F>(Arc::new(move |data| {
            consumer_clone.send(splice_fn(data)?)
        })));

        // Set up a `Link` from the producer to the `SpliceTransport<F>`
        let link = Arc::new(Pipeline::Link(
            producer.clone(),
            splice_transport.clone(),
            Arc::new(Task::new(|_, state| {
                    println!("Setting up splice");
                    let (producer, consumer) = state.clone().into_inner();
                    async move {
                        println!("running splice");
                        consumer.send(producer.recv()?)?;
                        Ok(())
                    }
                }, (producer, splice_transport as Arc<dyn Transport<F>>).as_task_state())),
        ));

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
    fn send(&self, data: F) -> Result<(), crate::TransportError> {
        self.0.send(data)
    }

    fn recv(&self) -> Result<F, crate::TransportError> {
        Err(crate::TransportError::NoRecv("Not receivable. Rather than `recv<F>`, must access the internal `splice.consumer<T>()` to call `recv<T>`.".to_string()))
    }
}

/* ********************
  SpliceTransport
******************** */
type SpliceFnWrapper<F> = Arc<dyn Fn(F) -> Result<(), TransportError> + Send + Sync>;

/// `SpliceTransport` acts as a hook to call the `SpliceFn` of the `Splice` that created it
pub struct SpliceTransport<F: TransportItemRequirements>(SpliceFnWrapper<F>);

impl<F: TransportItemRequirements> std::fmt::Debug for SpliceTransport<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SpliceTransport")
            .field(&"<SpliceFnWrapper>")
            .finish()
    }
}

/// On `send`, calls the `SpliceFn` of the `Splice` that created the `SpliceTransport`.
/// Not receivable. Must `recv` from the `Splice` internal consumer.
impl<F: TransportItemRequirements> Transport<F> for SpliceTransport<F> {
    fn send(&self, data: F) -> Result<(), crate::TransportError> {
        self.0(data)
    }

    fn recv(&self) -> Result<F, crate::TransportError> {
        Err(crate::TransportError::NoRecv(
            "Not receivable. Must `recv` from the `Splice` internal consumer.".to_string(),
        ))
    }
}
