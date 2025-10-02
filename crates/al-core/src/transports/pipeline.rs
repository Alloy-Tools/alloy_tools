use std::sync::Arc;

use crate::{ExtendedTaskState, Task, Transport, TransportError, TransportItemRequirements};

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
    fn send(&self, data: T) -> Result<(), TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.send(data),
            Pipeline::Transform(transport, transform_fn, _) => transport.send(transform_fn(data)),
            Pipeline::Link(transport, _, _) => transport.send(data),
        }
    }

    fn recv(&self) -> Result<T, TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.recv(),
            Pipeline::Transform(transport, _, transform_fn) => Ok(transform_fn(transport.recv()?)),
            Pipeline::Link(_, transport, _) => transport.recv(),
        }
    }
}
