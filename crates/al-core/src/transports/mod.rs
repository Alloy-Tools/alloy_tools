pub mod pipeline;
pub mod queue;
pub mod splice;

#[cfg(test)]
mod tests {
    use crate::{Command, Pipeline, Queue, Splice, Transport, TransportItemRequirements};
    use std::sync::Arc;

    fn make_queue_link<T: TransportItemRequirements>(
        t0: Option<Arc<dyn Transport<T>>>,
        t1: Option<Arc<dyn Transport<T>>>,
    ) -> Pipeline<T> {
        let t0 = t0.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        let t1 = t1.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        Pipeline::link(t0, t1)
    }

    #[tokio::test]
    async fn queue_debug() {
        assert_eq!(
            format!(
                "{:?}",
                Queue::<Command>::new()
            ),
            "Queue { queue: [] }"
        );
    }

    #[tokio::test]
    async fn pipeline_debug() {
        let l0 = make_queue_link::<Command>(None, None);
        assert_eq!(
            format!("{:?}", l0),
            "Link(Queue { queue: [] }, Queue { queue: [] }, \"<LinkFn>\")"
        );
        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );
        assert_eq!(format!("{:?}", l1), "Link(Link(Queue { queue: [] }, Queue { queue: [] }, \"<LinkFn>\"), Link(Queue { queue: [] }, Queue { queue: [] }, \"<LinkFn>\"), \"<LinkFn>\")");
    }

    #[tokio::test]
    async fn pipeline_send() {
        let l0 = make_queue_link(None, None);
        l0.send(Command::Stop).await.unwrap();
        assert_eq!(l0.recv().await.unwrap(), Command::Stop);

        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );

        l1.send(Command::Stop).await.unwrap();
        assert_eq!(l1.recv().await.unwrap(), Command::Stop);
    }

    #[tokio::test]
    async fn splice_pipeline_send() {
        // `Splice::new` inserts a `SpliceTransport` between the pipelines, returning the `Splice` as the transport
        let splice = Splice::new(
            Arc::new(make_queue_link::<Command>(None, None)),
            Arc::new(make_queue_link::<String>(None, None)),
            Arc::new(move |data| async move { Ok(format!("Command: {:?}", data)) }),
        );

        // Send `Command`
        splice.send(Command::Stop).await.unwrap();

        // Recv `String`
        assert_eq!(splice.consumer().recv().await.unwrap(), "Command: Stop");
    }
}
