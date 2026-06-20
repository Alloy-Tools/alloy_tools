use crate::{Action, Transport, TransportID, TransportIDError, TransportItemRequirements};
use al_structures::noop_waker::noop_waker;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DriverError {
    InvalidSender(TransportIDError),
    InvalidReceiver(TransportIDError),
    SelfConnection,
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverError::InvalidSender(id_error) => {
                write!(f, "Invalid TransportID for sender: {id_error}")
            }
            DriverError::InvalidReceiver(id_error) => {
                write!(f, "Invalid TransportID for receiver: {id_error}")
            }
            DriverError::SelfConnection => write!(f, "Transports cannot connect to themselves"),
        }
    }
}

impl<T: TransportItemRequirements> std::fmt::Debug for Driver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.transports.len() {
            write!(
                f,
                "{}{}",
                self.transports[i].status(),
                if i != self.transports.len() - 1 {
                    "\n"
                } else {
                    ""
                }
            )?
        }
        Ok(())
    }
}

pub struct Driver<T: TransportItemRequirements> {
    transports: Vec<Box<dyn Transport<T>>>,
    edges: Vec<Option<usize>>,         // 1:1 by move
    fan_outs: Vec<Option<Vec<usize>>>, // 1:n through clone
}

impl<T: TransportItemRequirements> std::future::Future for Driver<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.get_mut().poll(cx);
        std::task::Poll::Pending
    }
}

impl<T: TransportItemRequirements> Default for Driver<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TransportItemRequirements> Driver<T> {
    pub fn new() -> Self {
        Self {
            transports: Vec::new(),
            edges: Vec::new(),
            fan_outs: Vec::new(),
        }
    }

    pub async fn drive<F, Fut>(mut self, yield_fn: F)
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let waker = noop_waker();
        loop {
            self.poll(&mut std::task::Context::from_waker(waker));
            yield_fn().await;
        }
    }

    /// Validates the TransportID and returns `Some(transports_index)` for valid IDs
    fn resolve(&self, id: TransportID) -> Result<usize, TransportIDError> {
        //TODO: validate ID with StableVec
        if id.index < self.transports.len() {
            Ok(id.index)
        } else {
            Err(TransportIDError::InvalidIndex)
        }
    }

    pub fn add_transport(&mut self, transport: impl Into<Box<dyn Transport<T>>>) -> TransportID {
        //TODO: get all from StableVec once implemented with generations
        let index = self.transports.len();
        let generation = 0;

        self.transports.push(transport.into());
        self.edges.push(None);
        self.fan_outs.push(None);

        TransportID { index, generation }
    }

    pub fn connect(&mut self, from: TransportID, to: TransportID) -> Result<(), DriverError> {
        let fi = self.resolve(from).map_err(DriverError::InvalidSender)?;
        let ti = self.resolve(to).map_err(DriverError::InvalidReceiver)?;

        if fi == ti {
            Err(DriverError::SelfConnection)?
        }

        match self.edges[fi] {
            Some(e) => {
                // if already connected to one edge, which isn't the same one being added, move to fanout
                if e != ti {
                    self.edges[fi] = None;
                    self.fan_outs[fi] = Some(vec![e, ti]);
                }
            }
            None => match self.fan_outs[fi].as_mut() {
                Some(vec) => {
                    if !vec.contains(&ti) {
                        vec.push(ti)
                    }
                }
                None => self.edges[fi] = Some(ti),
            },
        }
        Ok(())
    }

    pub fn disconnect(&mut self, from: TransportID, to: TransportID) -> Result<(), DriverError> {
        let fi = self.resolve(from).map_err(DriverError::InvalidSender)?;
        let ti = self.resolve(to).map_err(DriverError::InvalidReceiver)?;

        if fi != ti {
            match self.edges[fi] {
                Some(e) => {
                    if e == ti {
                        self.edges[fi] = None;
                    }
                }
                None => {
                    if let Some(vec) = self.fan_outs[fi].as_mut() {
                        if let Some(index) = vec.iter().position(|&id| id == ti) {
                            vec.swap_remove(index);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn send_from(
        &mut self,
        from_idx: usize,
        data: T,
        work: &mut std::collections::VecDeque<usize>,
    ) {
        if let Some(to_idx) = self.edges[from_idx] {
            self.transports[to_idx].handle_incoming(data);
            if self.edges[to_idx].is_some() || self.fan_outs[to_idx].is_some() {
                work.push_back(to_idx);
            }
        } else if let Some(ref to_idxs) = self.fan_outs[from_idx] {
            for &index in to_idxs {
                self.transports[index].handle_incoming(data.clone());
                if self.edges[index].is_some() || self.fan_outs[index].is_some() {
                    work.push_back(index);
                }
            }
        }
    }

    pub fn deliver_to(&mut self, to: TransportID, data: T) -> Result<(), TransportIDError> {
        let ti = self.resolve(to)?;
        self.transports[ti].handle_incoming(data);
        Ok(())
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) {
        let mut work = std::collections::VecDeque::new();

        for i in 0..self.transports.len() {
            if self.edges[i].is_some() || self.fan_outs[i].is_some() {
                work.push_back(i);
            }
        }

        while let Some(index) = work.pop_front() {
            if let std::task::Poll::Ready(Action::Data(data)) =
                self.transports[index].poll_action(cx)
            {
                self.send_from(index, data, &mut work);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CountingConsumer, CountingProducer};

    #[test]
    fn connect_self() {
        let mut driver = Driver::new();
        let id = driver.add_transport(CountingProducer::new());

        assert!(matches!(
            driver.connect(id, id),
            Err(DriverError::SelfConnection)
        ));
    }

    #[test]
    fn invalid_sender() {
        let mut driver = Driver::new();
        let valid_id = driver.add_transport(CountingProducer::new());
        let invalid_id = TransportID {
            index: 1,
            generation: 0,
        };

        assert!(matches!(
            driver.connect(invalid_id, valid_id),
            Err(DriverError::InvalidSender(TransportIDError::InvalidIndex))
        ));
    }

    #[test]
    fn invalid_receiver() {
        let mut driver = Driver::new();
        let valid_id = driver.add_transport(CountingProducer::new());
        let invalid_id = TransportID {
            index: 1,
            generation: 0,
        };

        assert!(matches!(
            driver.connect(valid_id, invalid_id),
            Err(DriverError::InvalidReceiver(TransportIDError::InvalidIndex))
        ));
    }

    #[test]
    fn deliver_to_invalid_id() {
        let mut driver = Driver::new();
        let invalid_id = TransportID {
            index: 1,
            generation: 0,
        };

        assert!(matches!(
            driver.deliver_to(invalid_id, 10),
            Err(TransportIDError::InvalidIndex)
        ));
    }

    #[test]
    fn disconnect() {
        let mut driver = Driver::new();
        let producer = driver.add_transport(CountingProducer::new());
        let consumer = driver.add_transport(CountingConsumer::new());
        driver.connect(producer, consumer).unwrap();
        assert!(format!("{:?}", driver).contains("0"));

        let waker = noop_waker();
        let mut cx = std::task::Context::from_waker(waker);
        driver.poll(&mut cx);
        assert!(format!("{:?}", driver).contains("1"));

        assert!(driver.disconnect(producer, consumer).is_ok());
        driver.poll(&mut cx);
        assert!(format!("{:?}", driver).contains("1"))
    }

    #[test]
    fn disconnect_nonexistent() {
        let mut driver = Driver::new();
        let producer = driver.add_transport(CountingProducer::new());
        let consumer = driver.add_transport(CountingConsumer::new());

        assert!(driver.disconnect(producer, consumer).is_ok());
    }

    #[test]
    fn disconnect_invalid_sender() {
        let mut driver = Driver::new();
        let valid_id = driver.add_transport(CountingConsumer::new());
        let invalid_id = TransportID {
            index: 1,
            generation: 0,
        };

        assert!(matches!(
            driver.disconnect(invalid_id, valid_id),
            Err(DriverError::InvalidSender(TransportIDError::InvalidIndex))
        ));
    }

    #[test]
    fn deliver_to() {
        let mut driver = Driver::new();
        let consumer = driver.add_transport(CountingConsumer::new());

        assert!(driver.deliver_to(consumer, 10).is_ok());
    }

    #[test]
    fn multiple_connections() {
        let mut driver = Driver::new();
        let producer = driver.add_transport(CountingProducer::new());
        let consumer1 = driver.add_transport(CountingConsumer::new());
        let consumer2 = driver.add_transport(CountingConsumer::new());

        // Connect producer to both consumers
        driver.connect(producer, consumer1).unwrap();
        driver.connect(producer, consumer2).unwrap();

        // After polling, both should receive data
        let waker = noop_waker();
        let mut cx = std::task::Context::from_waker(waker);
        driver.poll(&mut cx);

        assert_eq!(
            &format!("{:?}", driver),
            "Next Value: 1\nItems received: 1\nItems received: 1"
        )
    }
}
