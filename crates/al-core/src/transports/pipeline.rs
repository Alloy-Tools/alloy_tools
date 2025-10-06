use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    ExtendedTaskState, Task, Transport, TransportError, TransportItemRequirements, WithTaskState,
};

pub trait TransformFn<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + Sync> TransformFn<T> for F {}

type LinkTask<T> = Arc<
    Task<
        (),
        TransportError,
        ExtendedTaskState<(), TransportError, (Arc<dyn Transport<T>>, Arc<dyn Transport<T>>)>,
    >,
>;

//TODO: Can `Pipeline::Transport` be removed? can internals be `&'a dyn Transport<T>` rather than `Arc`?
/// Recursive `Pipeline<T>` enum allowing multiple `dyn Transport<T>` to be combined together into a single `Pipeline<T>`.
#[derive(Clone)]
pub enum Pipeline<T: TransportItemRequirements> {
    Transport(Arc<dyn Transport<T>>),
    Transform(
        Arc<dyn Transport<T>>,
        Arc<dyn TransformFn<T>>,
        Arc<dyn TransformFn<T>>,
    ),
    Link(Arc<dyn Transport<T>>, Arc<dyn Transport<T>>, LinkTask<T>),
}

impl<T: TransportItemRequirements> Pipeline<T> {
    /// Creates a new `Pipeline::Link` with a `Task` handling the connection from the `producer` to the `consumer`
    pub fn link(producer: Arc<dyn Transport<T>>, consumer: Arc<dyn Transport<T>>) -> Pipeline<T> {
        Pipeline::Link(
            producer.clone(),
            consumer.clone(),
            Arc::new(Task::infinite(
                {
                    move |_,
                          state: &Arc<
                        RwLock<
                            crate::ExtendedTaskState<
                                (),
                                crate::TransportError,
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
        )
    }
}

/// Impl Debug for pipeline manually as `Fn()` doesn't support `Debug`
impl<T: TransportItemRequirements> std::fmt::Debug for Pipeline<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(transport) => f.debug_tuple("Transport").field(transport).finish(),
            Self::Transform(pipeline, _, _) => f
                .debug_tuple("Transform")
                .field(pipeline)
                .field(&"<TransformFn>")
                .field(&"<TransformFn>")
                .finish(),
            Self::Link(pipeline0, pipeline1, _) => f
                .debug_tuple("Link")
                .field(pipeline0)
                .field(pipeline1)
                .field(&"<LinkFn>")
                .finish(),
        }
    }
}

/// Impl `Transport` for `Pipeline`, handling each variant case for `send` and `recv`
impl<T: TransportItemRequirements> Transport<T> for Pipeline<T> {
    fn send_blocking(&self, data: T) -> Result<(), TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.send_blocking(data),
            Pipeline::Transform(transport, transform_fn, _) => {
                transport.send_blocking(transform_fn(data))
            }
            Pipeline::Link(transport, _, _) => transport.send_blocking(data),
        }
    }

    fn recv_blocking(&self) -> Result<T, TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.recv_blocking(),
            Pipeline::Transform(transport, _, transform_fn) => {
                Ok(transform_fn(transport.recv_blocking()?))
            }
            Pipeline::Link(_, transport, _) => transport.recv_blocking(),
        }
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
        match self {
            Pipeline::Transport(transport) => transport.send(data),
            Pipeline::Transform(transport, transform_fn, _) => transport.send(transform_fn(data)),
            Pipeline::Link(transport, _, _) => transport.send(data),
        }
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
        match self {
            Pipeline::Transport(transport) => transport.recv(),
            Pipeline::Transform(transport, _, transform_fn) => {
                Box::pin(async { Ok(transform_fn(transport.recv().await?)) })
            }
            Pipeline::Link(_, transport, _) => transport.recv(),
        }
    }
}
