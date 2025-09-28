use std::sync::Arc;

use crate::{Pipeline, Transport, TransportRequirements};

pub trait SpliceFn<T, U>: Fn(T) -> U + Send + Sync {}
impl<T: TransportRequirements, U: TransportRequirements, F: Fn(T) -> U + Send + Sync> SpliceFn<T, U>
    for F
{
}

pub struct Splice<F: TransportRequirements, T: TransportRequirements> {
    producer: Arc<Pipeline<F>>,
    consumer: Arc<Pipeline<T>>,
    splice_fn: Arc<dyn SpliceFn<F, T>>,
}

impl<F: TransportRequirements, T: TransportRequirements> std::fmt::Debug for Splice<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Splice")
            .field("from", &crate::event::type_with_generics(&self.producer))
            .field("to", &crate::event::type_with_generics(&self.consumer))
            .finish()
    }
}

impl<F: TransportRequirements, T: TransportRequirements> Splice<F, T> {
    pub fn new(
        producer: Arc<Pipeline<F>>,
        consumer: Arc<Pipeline<T>>,
        splice_fn: Arc<dyn SpliceFn<F, T>>,
    ) -> (Arc<Pipeline<F>>, Arc<Pipeline<T>>) {
        let ret = (producer.clone(), consumer.clone());
        let splice = Arc::new(Self {
            producer,
            consumer,
            splice_fn,
        });

        //TODO: insert the new `SpliceTransport` into the end of the producer (the consumer shouldn't need it?? its not recv-able anyways)
        // Then return the new `Splice` that acts as the pipeline wrapper

        ret
    }
}

/// On `send`, converts data from type `F` to type `T` using the provided `SpliceFn<F,T>` and calls `send` on the consumer.
/// Not receivable.
impl<F: TransportRequirements, T: TransportRequirements> Transport<F> for Splice<F, T> {
    fn send(&self, data: F) -> Result<(), crate::TransportError> {
        self.consumer.send((self.splice_fn)(data))
    }

    fn recv(&self) -> Result<F, crate::TransportError> {
        Err(crate::TransportError::NoRecv)
    }
}

/* TODO:
`Splice` should wrap the pipelines, making `Splice.send` send to the producer while `Splice.recv` recvs from the consumer
`SpliceTransport` should do what `Splice` did. Not recv-able and send should send to the consumer after using the `splice_fn`
*/
pub struct SpliceTransport<T: TransportRequirements> {
    producer: Arc<Pipeline<F>>,
    consumer: Arc<Pipeline<T>>,
    splice_fn: Arc<dyn SpliceFn<F, T>>,
}
