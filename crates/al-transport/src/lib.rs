//! # Driver
//!
//! The `Driver` is the coordinator of the transport system. It manages multiple transports
//! and routes data between them based on configurable connections.
//!
//! ## Basic Usage
//!
//! ```ignore
//! let mut driver = Driver::new();
//!
//! let producer = driver.add_transport(MyProducer::new());
//! let consumer = driver.add_transport(MyConsumer::new());
//!
//! driver.connect(producer, consumer)?;
//!
//! // Drive the system asynchronously
//! tokio::spawn(async move {
//!     driver.drive(|| tokio::task::yield_now()).await;
//! });
//! ```
//!
//! ## Error Handling
//!
//! - `DriverError::InvalidSender` / `InvalidReceiver`: Returned when a `TransportID` doesn't match any registered transport
//! - `DriverError::SelfConnection`: Returned when trying to connect a transport to itself
//!
//! # Splice Functions
//!
//! Utilities for bridging between sync and async queue boundaries.
//!
//! Splice functions spawn independent tasks or threads that continuously read from a source queue,
//! apply a transformation, and write to a destination queue. They're useful for adapting between
//! different concurrency models (sync ↔ async).
//!
//! ## Features
//!
//! - **Dual-mode support**: Both blocking (std::thread) and async (tokio) variants
//! - **Cancellation**: Returns [`SpliceHandle`] to gracefully stop the operation
//! - **Transformation**: Applies a closure to items during transit
//! - **Fire-and-forget**: Spawns independently; caller can drop handle safely
//!
//! ## Examples
//!
//! ### Async Splice (requires `tokio` feature)
//!
//! ```ignore
//! use al_transport::{BoundaryQueue, splice_async};
//!
//! let source = BoundaryQueue::new();
//! let dest = BoundaryQueue::new();
//!
//! let handle = splice_async(source.clone(), dest.clone(), |x: i32| x * 2).await;
//!
//! // Can stop later:
//! handle.stop();
//! ```
//!
//! ### Blocking Splice
//!
//! ```ignore
//! use al_transport::{BoundaryQueue, splice_blocking};
//!
//! let source = BoundaryQueue::new();
//! let dest = BoundaryQueue::new();
//!
//! let handle = splice_blocking(source.clone(), dest.clone(), |x: i32| x * 2);
//!
//! // Do other work...
//! handle.stop();  // Clean shutdown
//! ```
//!
//! ## Error Handling
//!
//! Both functions use `.expect()` on queue operations; panics indicate a critical failure
//! (poisoned mutex or condition variable failure). This is acceptable as failures here suggest
//! a serious system-level issue that can't be recovered from gracefully.
//!
//! ## Performance Notes
//!
//! - Each splice spawns a new task/thread (overhead proportional to spawn cost)
//! - Suitable for medium-frequency data flows; consider batching for high throughput
//! - AtomicBool stop flag uses SeqCst for strong ordering guarantees

//#![deny(missing_docs)]

//REVIEW: Instead of requiring clone and cloning/moving data around, the dependencey inversion already
//      made everything sync and get owned by the driver, so look into using Rc (the thread local version of Arc)
//      (or even just passing a ref `&_` with owned data? as the code is sync so a `&_` could be passed when ran?)
//      to have some sort of reuseable buffer of the T data type. Then on recv from a `BoundaryQueue`
//      the data would get inserted into the data buffer and its buffer id/reference would be passed instead?
//      That would make the "actual" data movement easier as it wouldn't move but just be referenced?
//      But then what if the stage needs to modify it? Since its sync it should be safe to get mut.
//      Then on send to a `BoundaryQueue` it could send it if Rc < 2 or clone and drop ref?
//      So I would still need clone but it would just make the system do less work?
//      I need to estimate the benefit of not actually moving/cloning the data vs the cost of Rc and getting mut.
//      Copies might be treated differently so I might need a CoW instead of Rc.

mod driver;
mod marker;
pub mod splice;
mod transport;
pub mod transports;

pub use driver::{Driver, DriverError};
pub use marker::TransportItemRequirements;
#[cfg(test)]
pub use test_counting::{CountingConsumer, CountingProducer};
pub use transport::{Action, Transport, TransportID, TransportIDError};

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
    use crate::transports::*;
    use al_structures::noop_waker::noop_waker;

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
