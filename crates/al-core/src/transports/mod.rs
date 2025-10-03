pub mod pipeline;
pub mod queue;
pub mod splice;

#[cfg(test)]
mod tests {
    use tokio::time::sleep;

    use crate::{
        task::WithTaskState, Command, Pipeline, Queue, Splice, Task, Transport,
        TransportItemRequirements,
    };
    use std::{sync::Arc, time::Duration};

    fn make_queue_link<T: TransportItemRequirements>(
        t0: Option<Arc<dyn Transport<T>>>,
        t1: Option<Arc<dyn Transport<T>>>,
    ) -> Pipeline<T> {
        let t0 = t0.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        let t1 = t1.unwrap_or_else(|| Arc::new(Queue::<T>::new()));
        Pipeline::Link(
            t0.clone(),
            t1.clone(),
            Arc::new(Task::new(
                {
                    println!("Setting up passed fn");
                    move |_,
                          state: &Arc<
                        tokio::sync::RwLock<
                            crate::ExtendedTaskState<
                                (),
                                crate::TransportError,
                                (Arc<dyn Transport<T>>, Arc<dyn Transport<T>>),
                            >,
                        >,
                    >| {
                        println!("Running outer passed fn");
                        let (t0, t1) = state.blocking_read().inner_clone();
                        async move {
                            println!("Running inner passed fn");
                            let data = t0.recv()?;
                            println!("Got data: {:?}", data);
                            t1.send(data)?;
                            Ok(())
                        }
                    }
                },
                (t0, t1).as_task_state(),
            )),
        )
    }

    #[tokio::test]
    async fn pipeline_debug() {
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

    #[tokio::test]
    async fn pipeline_send() {
        let l0 = make_queue_link(None, None);
        l0.send(Command::Stop).unwrap();
        sleep(Duration::from_secs(2)).await;
        assert_eq!(l0.recv().unwrap(), Command::Stop);

        let l1 = make_queue_link(
            Some(Arc::new(l0)),
            Some(Arc::new(make_queue_link(None, None))),
        );

        sleep(Duration::from_secs(4)).await;

        l1.send(Command::Stop).unwrap();
        assert_eq!(l1.recv().unwrap(), Command::Stop);
    }

    #[tokio::test]
    async fn splice_pipeline_send() {
        // `Splice::new` inserts a `SpliceTransport` between the pipelines, returning the `Splice` as the transport
        let splice = Splice::new(
            Arc::new(make_queue_link::<Command>(None, None)),
            Arc::new(make_queue_link::<String>(None, None)),
            Arc::new(move |data| {
                let ret = format!("Command: {:?}", data);
                println!("{}", ret);
                Ok(ret)
            }),
        );

        // Send `Command`
        splice.send(Command::Stop).unwrap();

        // Recv `String`
        assert_eq!(splice.consumer().recv().unwrap(), "Command: Command::Stop");
    }
}
