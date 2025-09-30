use std::sync::Arc;

use crate::{Transport, TransportError, TransportRequirements};

pub trait TransformFn<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportRequirements, F: Fn(T) -> T + Send + Sync> TransformFn<T> for F {}
pub trait LinkFn: Fn() -> Result<(), TransportError> + Send + Sync {}
impl<F: Fn() -> Result<(), TransportError> + Send + Sync> LinkFn for F {}

//TODO: Internals are `Arc(Pipeline<T>)` rather than `Arc(dyn Transport<T>)` to enable pattern matching. Remove once `Link`s are setup with `Watcher`s rather than manually polling. Can `Pipeline::Transport` be removed then?
//TODO: can internals be `&'a dyn Transport<T>` rather than `Arc`?
/// Recursive `Pipeline<T>` enum allowing multiple `dyn Transport<T>` to be combined together into a single `Pipeline<T>`.
#[derive(Clone)]
pub enum Pipeline<T: TransportRequirements> {
    Transport(Arc<dyn Transport<T>>),
    Transform(
        Arc<Pipeline<T>>,
        Arc<dyn TransformFn<T>>,
        Arc<dyn TransformFn<T>>,
    ),
    Link(Arc<Pipeline<T>>, Arc<Pipeline<T>>, Arc<dyn LinkFn>),
}

/// Impl Debug for pipeline manually as `Fn()` doesn't support `Debug`
impl<T: TransportRequirements> std::fmt::Debug for Pipeline<T> {
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
impl<T: TransportRequirements> Transport<T> for Pipeline<T> {
    fn send(&self, data: T) -> Result<(), TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.send(data),
            Pipeline::Transform(pipeline, transform_fn, _) => pipeline.send(transform_fn(data)),
            Pipeline::Link(pipeline, _, _) => pipeline.send(data),
        }
    }

    fn recv(&self) -> Result<T, TransportError> {
        match self {
            Pipeline::Transport(transport) => transport.recv(),
            Pipeline::Transform(pipeline, _, transform_fn) => Ok(transform_fn(pipeline.recv()?)),
            Pipeline::Link(_, pipeline, _) => pipeline.recv(),
        }
    }
}
