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

    /// Get the producer transport
    pub fn producer(&self) -> &Arc<dyn Transport<T>> {
        &self.producer
    }

    /// Get the consumer transport
    pub fn consumer(&self) -> &Arc<dyn Transport<T>> {
        &self.consumer
    }

    /// Get the link task
    pub fn link_task(&self) -> &LinkTask<T> {
        &self.link_task
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

    fn send_batch_blocking(&self, data: Vec<T>) -> Result<(), TransportError> {
        self.producer.send_batch_blocking(data)
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

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        self.producer.send_batch(data)
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

#[cfg(test)]
mod tests {
    use crate::{Link, Queue, Transport, TransportItemRequirements};
    use std::sync::Arc;

    fn make_link<T: TransportItemRequirements>(
        t0: Option<Arc<dyn Transport<T>>>,
        t1: Option<Arc<dyn Transport<T>>>,
    ) -> Link<T> {
        let t0 = t0.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        let t1 = t1.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        Link::new(t0, t1)
    }

    #[tokio::test]
    async fn debug() {
        let l0 = make_link::<u8>(None, None);
        assert_eq!(
            format!("{:?}", l0),
            "Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }"
        );
        let l1 = make_link(Some(Arc::new(l0)), Some(Arc::new(make_link(None, None))));
        assert_eq!(format!("{:?}", l1), "Link { producer: Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }, consumer: Link { producer: Queue { queue: [] }, consumer: Queue { queue: [] }, link_task: \"<LinkFn>\" }, link_task: \"<LinkFn>\" }");
    }

    #[tokio::test]
    async fn send_recv() {
        let l0 = make_link(None, None);
        l0.send(1u8).await.unwrap();
        l0.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(l0.recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(l0.try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(l0.recv_avaliable().await.unwrap(), vec![3]);

        l0.send_blocking(1).unwrap();
        l0.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(l0.recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(l0.try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(l0.recv_avaliable_blocking().unwrap(), vec![3]);

        let l1 = make_link(Some(Arc::new(l0)), Some(Arc::new(make_link(None, None))));
        l1.send(1).await.unwrap();
        l1.send_batch(vec![2, 3]).await.unwrap();
        // Recv `String` asynchronously
        assert_eq!(l1.recv().await.unwrap(), 1);
        // Try recv `String` asynchronously
        assert_eq!(l1.try_recv().await.unwrap().unwrap(), 2);
        // Recv avaliable `String` asynchronously
        assert_eq!(l1.recv_avaliable().await.unwrap(), vec![3]);

        l1.send_blocking(1).unwrap();
        l1.send_batch_blocking(vec![2, 3]).unwrap();
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        // Recv `String` synchronously
        assert_eq!(l1.recv_blocking().unwrap(), 1);
        // Try recv `String` synchronously
        assert_eq!(l1.try_recv_blocking().unwrap().unwrap(), 2);
        // Recv avaliable `String` synchronously
        assert_eq!(l1.recv_avaliable_blocking().unwrap(), vec![3]);
    }
}
