use crate::TransportRequirements;
use std::{
    fmt::Debug,
    sync::{Arc, PoisonError},
};

pub trait Transport<T>: TransportRequirements {
    fn send(&self, data: T) -> Result<(), TransportError>;
    fn recv(&self) -> Result<T, TransportError>;
}

#[derive(Debug)]
pub enum TransportError {
    Custom(String),
    LockPoisoned(String),
    NoSend,
    NoRecv,
}

impl<T> From<PoisonError<T>> for TransportError {
    fn from(err: PoisonError<T>) -> Self {
        TransportError::LockPoisoned(err.to_string())
    }
}

pub trait TransformFn<T>: Fn(T) -> T + Send + Sync {}
impl<T: TransportRequirements, U: Fn(T) -> T + Send + Sync> TransformFn<T> for U {}
pub trait LinkFn: Fn() -> Result<(), TransportError> + Send + Sync {}
impl<T: Fn() -> Result<(), TransportError> + Send + Sync> LinkFn for T {}

#[derive(Clone)]
pub enum Pipeline<T> {
    Transport(Arc<dyn Transport<T>>),
    Transform(
        Arc<Pipeline<T>>,
        Arc<dyn TransformFn<T>>,
        Arc<dyn TransformFn<T>>,
    ),
    //TODO: maybe an external (non `Pipeline`) Link that can take `Pipeline<T>` and `Pipeline<U>`. eg `Merge<T, U>(Pipeline<T>, Pipeline<U>, dyn LinkFn)`
    Link(Arc<Pipeline<T>>, Arc<Pipeline<T>>, Arc<dyn LinkFn>),
}

/// Impl Debug for pipeline manually as `Fn()` doesn't support `Debug`
impl<T> Debug for Pipeline<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //f.write_str(data);
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

#[cfg(test)]
mod test {
    use crate::{transport::Transport, Command, Pipeline, Queue, TransportRequirements};
    use std::sync::Arc;

    fn make_queue_link<T: TransportRequirements>(
        t0: Option<Arc<Pipeline<T>>>,
        t1: Option<Arc<Pipeline<T>>>,
    ) -> Pipeline<T> {
        let t0 = t0.unwrap_or_else(|| Arc::new(Pipeline::Transport(Arc::new(Queue::<T>::new()))));
        let t1 = t1.unwrap_or_else(|| Arc::new(Pipeline::Transport(Arc::new(Queue::<T>::new()))));
        let t0_clone = Arc::clone(&t0);
        let t1_clone = Arc::clone(&t1);
        Pipeline::Link(t0, t1, Arc::new(move || t1_clone.send(t0_clone.recv()?)))
    }

    #[test]
    fn pipeline_debug() {
        assert_eq!(
            format!(
                "{:?}",
                Pipeline::Transport(Arc::new(Queue::<Command>::new()))
            ),
            "Transport(Queue { queue: [] })"
        );
        let l0 = make_queue_link::<Command>(None, None);
        assert_eq!(
            format!("{:?}", l0),
            "Link(Transport(Queue { queue: [] }), Transport(Queue { queue: [] }), \"<LinkFn>\")"
        );
        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );
        assert_eq!(format!("{:?}", l1), "Link(Link(Transport(Queue { queue: [] }), Transport(Queue { queue: [] }), \"<LinkFn>\"), Link(Transport(Queue { queue: [] }), Transport(Queue { queue: [] }), \"<LinkFn>\"), \"<LinkFn>\")");
    }

    #[test]
    fn pipeline_send() {
        let l0 = make_queue_link(None, None);
        l0.send(Command::Stop).unwrap();
        if let Pipeline::Link(_, _, ref link_fn) = l0 {
            link_fn().unwrap();
        } else {
            panic!("The pipeline should be a link!");
        }
        assert_eq!(l0.recv().unwrap(), Command::Stop);

        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );

        l1.send(Command::Stop).unwrap();
        if let Pipeline::Link(ref pipeline0, ref pipeline1, ref link_fn) = l1 {
            if let Pipeline::Link(_, _, ref link_fn) = **pipeline0 {
                link_fn().unwrap();
            } else {
                panic!("The pipeline should be a link!");
            }
            link_fn().unwrap();
            if let Pipeline::Link(_, _, ref link_fn) = **pipeline1 {
                link_fn().unwrap();
            } else {
                panic!("The pipeline should be a link!");
            }
        } else {
            panic!("The pipeline should be a link!");
        }
        assert_eq!(l1.recv().unwrap(), Command::Stop);
    }

    #[test]
    fn mixed_pipeline_send() {
        todo!()
    }
}
