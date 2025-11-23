use crate::{
    AsTaskState, ExtendedTaskState, Task, Transport, TransportError, TransportItemRequirements,
};
use std::sync::Arc;
use tokio::sync::RwLock;

type LinkTask<T> = Arc<
    Task<
        (),
        TransportError,
        ExtendedTaskState<(), TransportError, (Arc<dyn Transport<T>>, Arc<dyn Transport<T>>)>,
    >,
>;

pub struct Link<T: TransportItemRequirements> {
    producer: Arc<dyn Transport<T>>,
    consumer: Arc<dyn Transport<T>>,
    #[allow(unused)]
    link_task: LinkTask<T>,
}

impl<T: TransportItemRequirements> From<Link<T>> for Arc<dyn Transport<T>> {
    fn from(link: Link<T>) -> Self {
        Arc::new(link)
    }
}

impl<T: TransportItemRequirements> Link<T> {
    pub fn with_task(
        producer: Arc<dyn Transport<T>>,
        consumer: Arc<dyn Transport<T>>,
        link_task: LinkTask<T>,
    ) -> Self {
        Self {
            producer,
            consumer,
            link_task,
        }
    }

    /// Creates a new `Link` with a `Task` handling the connection from the `producer` to the `consumer`
    pub fn new(producer: Arc<dyn Transport<T>>, consumer: Arc<dyn Transport<T>>) -> Self {
        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            link_task: Arc::new(Task::infinite(
                {
                    |_,
                     state: &Arc<
                        RwLock<
                            ExtendedTaskState<
                                (),
                                TransportError,
                                (Arc<dyn Transport<T>>, Arc<dyn Transport<T>>),
                            >,
                        >,
                    >| {
                        let state = state.clone();
                        async move {
                            let (producer, consumer) = state.read().await.inner_clone();
                            // This tight inner loop ignores errors and never ends, meaning the above clones only happen on the first iteration
                            // This means any `Task` a `Link` starts will only stop after `task.abort()`
                            loop {
                                if let Ok(data) = producer.recv().await {
                                    let _ = consumer.send(data).await;
                                }
                            }
                        }
                    }
                },
                (producer, consumer).as_task_state(),
            )),
        }
    }
}

/// Impl Debug for link manually as `Fn()` doesn't support `Debug`
impl<T: TransportItemRequirements> std::fmt::Debug for Link<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("producer", &self.producer)
            .field("consumer", &self.consumer)
            .field("link_task", &"<LinkFn>")
            .finish()
    }
}

impl<T: TransportItemRequirements> Transport<T> for Link<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        self.producer.send_blocking(data)
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        self.consumer.recv_blocking()
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, TransportError> {
        self.consumer.recv_avaliable_blocking()
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, TransportError> {
        self.consumer.try_recv_blocking()
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
        self.producer.send(data)
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
        self.consumer.recv()
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
        self.consumer.recv_avaliable()
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
        self.consumer.try_recv()
    }
}
