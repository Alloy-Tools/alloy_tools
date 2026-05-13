mod driver;
mod marker;
mod noop_waker;
mod splice;
mod transport;
mod transports;

pub use driver::{Driver, DriverError};
pub use marker::TransportItemRequirements;
pub use noop_waker::{new_noop_waker, noop_context, noop_waker, NoOpWaker};
pub use splice::{splice_async, splice_blocking};
#[cfg(test)]
pub use test_counting::{CountingConsumer, CountingProducer};
pub use transport::{Action, Transport, TransportID, TransportIDError};
pub use transports::{
    boundary_queue::{BoundaryQueue, BoundaryQueueError, BoundaryQueueTransport},
    filter::Filter,
    map::Map,
    queue::Queue,
};

#[cfg(test)]
mod test_counting {
    use super::*;

    pub struct CountingProducer(u64);
    impl CountingProducer {
        pub fn new() -> Self {
            CountingProducer(0)
        }
    }
    impl Transport<u64> for CountingProducer {
        fn handle_incoming(&mut self, _: u64) {}
        fn poll_action(&mut self, _: &mut std::task::Context<'_>) -> std::task::Poll<Action<u64>> {
            let val = self.0;
            self.0 = self.0.saturating_add(1);
            std::task::Poll::Ready(Action::Data(val))
        }
        fn status(&self) -> String {
            format!("Next Value: {}", self.0)
        }
    }

    impl From<CountingProducer> for Box<dyn Transport<u64>> {
        fn from(value: CountingProducer) -> Self {
            Box::new(value)
        }
    }

    pub struct CountingConsumer(u64);
    impl CountingConsumer {
        pub fn new() -> Self {
            CountingConsumer(0)
        }
        #[allow(dead_code)]
        pub fn count(&self) -> u64 {
            self.0
        }
    }
    impl Transport<u64> for CountingConsumer {
        fn handle_incoming(&mut self, _: u64) {
            self.0 += 1;
        }
        fn poll_action(&mut self, _: &mut std::task::Context<'_>) -> std::task::Poll<Action<u64>> {
            std::task::Poll::Pending
        }
        fn status(&self) -> String {
            format!("Items received: {}", self.0)
        }
    }

    impl From<CountingConsumer> for Box<dyn Transport<u64>> {
        fn from(value: CountingConsumer) -> Self {
            Box::new(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_driver() -> (Driver<u64>, std::sync::Arc<BoundaryQueue<u64>>) {
        let mut driver = Driver::new();

        let producer = driver.add_transport(CountingProducer::new());
        let square_map = driver.add_transport(Map::new(|x| x * x));
        let output = BoundaryQueue::new();
        let output_id = driver.add_transport(output.clone());

        driver.connect(producer, square_map).unwrap();
        driver.connect(square_map, output_id).unwrap();
        (driver, output)
    }

    #[test]
    fn sync() {
        let (mut driver, output) = setup_driver();

        let waker = noop_waker();
        for i in 0..10 {
            let mut cx = std::task::Context::from_waker(waker);
            driver.poll(&mut cx);

            let val = output
                .try_recv()
                .expect("failed to try_recv")
                .expect("no data");
            assert_eq!(val, i * i, "Mismatch on iteration {i}");
            println!("{val}");
        }
    }

    #[tokio::test]
    async fn asynchronous() {
        let (driver, output) = setup_driver();

        tokio::spawn(async move {
            driver.drive(|| tokio::task::yield_now()).await;
        });

        for i in 0..10 {
            let val = output.recv().await.expect("failed to recv");
            assert_eq!(val, i * i, "Mismatch on iteration {i}");
            println!("{val}")
        }
    }
}
