pub mod link;
pub mod list;
pub mod publisher;
pub mod queue;
pub mod splice;
pub mod transform;

#[cfg(test)]
mod tests {
    use crate::{Command, Link, Queue, Splice, Transport, TransportItemRequirements};
    use std::sync::Arc;

    fn make_queue_link<T: TransportItemRequirements>(
        t0: Option<Arc<dyn Transport<T>>>,
        t1: Option<Arc<dyn Transport<T>>>,
    ) -> Link<T> {
        let t0 = t0.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        let t1 = t1.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        Link::new(t0, t1)
    }

    #[tokio::test]
    async fn queue_debug() {
        assert_eq!(
            format!("{:?}", Queue::<Command>::new()),
            "Queue { queue: [] }"
        );
    }

    #[tokio::test]
    async fn pipeline_debug() {
        let l0 = make_queue_link::<Command>(None, None);
        assert_eq!(
            format!("{:?}", l0),
            "Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }"
        );
        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );
        assert_eq!(format!("{:?}", l1), "Link { producer: Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }, consumer: Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }, link_task: \"<LinkFn>\" }");
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
            Arc::new(|data| Ok(format!("Command: {:?}", data))),
            Arc::new(|data| async move { Ok(format!("Command: {:?}", data)) }),
        );

        // Send `Command` asynchronously
        splice.send(Command::Stop).await.unwrap();

        // Recv `String` asynchronously
        assert_eq!(splice.consumer().recv().await.unwrap(), "Command: Stop");

        // Send `Command` synchronously
        splice.send_blocking(Command::Stop).unwrap();

        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;

        // Recv `String` synchronously
        assert_eq!(splice.consumer().recv_blocking().unwrap(), "Command: Stop");
    }
}
