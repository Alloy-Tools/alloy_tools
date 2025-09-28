pub mod pipeline;
pub mod queue;
pub mod splice;


#[cfg(test)]
mod test {
    use crate::{transport::Transport, transports::splice::Splice, Command, Pipeline, Queue, TransportRequirements};
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
    fn splice_pipeline_send() {
        let p0 = Arc::new(make_queue_link::<Command>(None, None));
        let p1 = Arc::new(make_queue_link::<String>(None, None));

        // Splice::new inserts a splice between the pipelines, returning the first pipeline as the entry to the splice
        let (p0, p1) = Splice::new(p0, p1, Arc::new(move |data| {
            let ret = format!("Command: {:?}", data);
            println!("{}", ret);
            ret
        }));

        // Send `Command`
        p0.send(Command::Stop).unwrap();
        // Handle first link
        if let Pipeline::Link(_, _, ref link_fn) = *p0 {
            link_fn().unwrap();
        } else {
            panic!("The pipeline should be a link!");
        }
        // Handle second link
        if let Pipeline::Link(_, _, ref link_fn) = *p1 {
            link_fn().unwrap();
        } else {
            panic!("The pipeline should be a link!");
        }
        // Recv `String`
        assert_eq!(p1.recv().unwrap(), "Command: Command::Stop");
    }
}