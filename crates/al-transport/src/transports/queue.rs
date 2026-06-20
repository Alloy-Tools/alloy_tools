//! # Queue Transport
//!
//! A simple in-memory FIFO queue transport for `Transport<T>` state machines.
//!
//! `Queue<T>` stores incoming items in a `VecDeque` and wakes a pending task when
//! new data arrives.
//!
//! ## Features
//!
//! - FIFO ordering
//! - Async wakeup support for pending polls
//! - Status reporting via `status()`
//!
//! ## Example
//!
//! ```ignore
//! use al_transport::Queue;
//!
//! let mut queue = Queue::<i32>::new();
//! queue.handle_incoming(42);
//!
//! let waker = crate::noop_waker().clone();
//! let mut cx = std::task::Context::from_waker(&waker);
//!
//! match queue.poll_action(&mut cx) {
//!     std::task::Poll::Ready(crate::Action::Data(value)) => println!("Got {}", value),
//!     _ => println!("Queue empty"),
//! }
//! ```

use crate::{Transport, TransportItemRequirements};
use std::{collections::VecDeque, task::Waker};

pub struct Queue<T> {
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T: TransportItemRequirements> From<Queue<T>> for Box<dyn Transport<T>> {
    fn from(value: Queue<T>) -> Self {
        Box::new(value)
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements> Transport<T> for Queue<T> {
    fn handle_incoming(&mut self, data: T) {
        self.queue.push_back(data);
        if let Some(w) = self.waker.take() {
            w.wake();
        }
    }

    fn poll_action(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<crate::Action<T>> {
        if let Some(data) = self.queue.pop_front() {
            std::task::Poll::Ready(crate::Action::Data(data))
        } else {
            self.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }

    fn status(&self) -> String {
        format!("Queue Length: {}", self.queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_structures::noop_waker::noop_waker;

    #[test]
    fn new_queue_is_empty() {
        let mut queue = Queue::<i32>::new();
        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        let result = queue.poll_action(&mut cx);
        if let std::task::Poll::Pending = result {
            // Expected
        } else {
            panic!("Expected Pending for empty queue");
        }
    }

    #[test]
    fn handle_incoming_then_poll() {
        let mut queue = Queue::new();
        queue.handle_incoming(42);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        let result = queue.poll_action(&mut cx);
        if let std::task::Poll::Ready(crate::Action::Data(data)) = result {
            assert_eq!(data, 42);
        } else {
            panic!("Expected Ready with data");
        }
    }

    #[test]
    fn multiple_items_fifo_order() {
        let mut queue = Queue::new();
        queue.handle_incoming(1);
        queue.handle_incoming(2);
        queue.handle_incoming(3);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        // Items should come out in FIFO order
        if let std::task::Poll::Ready(crate::Action::Data(data)) = queue.poll_action(&mut cx) {
            assert_eq!(data, 1);
        }
        if let std::task::Poll::Ready(crate::Action::Data(data)) = queue.poll_action(&mut cx) {
            assert_eq!(data, 2);
        }
        if let std::task::Poll::Ready(crate::Action::Data(data)) = queue.poll_action(&mut cx) {
            assert_eq!(data, 3);
        }
    }

    #[test]
    fn poll_empty_after_drain() {
        let mut queue = Queue::new();
        queue.handle_incoming(1);
        queue.handle_incoming(2);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        // Drain all items
        let _ = queue.poll_action(&mut cx);
        let _ = queue.poll_action(&mut cx);

        // Next poll should be Pending
        let result = queue.poll_action(&mut cx);
        if let std::task::Poll::Pending = result {
            // Expected
        } else {
            panic!("Expected Pending after draining");
        }
    }

    #[test]
    fn status_reports_queue_length() {
        let mut queue = Queue::new();
        assert_eq!(queue.status(), "Queue Length: 0");

        queue.handle_incoming(1);
        assert_eq!(queue.status(), "Queue Length: 1");

        queue.handle_incoming(2);
        queue.handle_incoming(3);
        assert_eq!(queue.status(), "Queue Length: 3");

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);
        let _ = queue.poll_action(&mut cx);

        assert_eq!(queue.status(), "Queue Length: 2");
    }

    #[test]
    fn waker_is_stored_when_empty() {
        let mut queue = Queue::<i32>::new();

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        // Poll on empty queue stores waker
        let _ = queue.poll_action(&mut cx);

        // Incoming data should wake it
        queue.handle_incoming(42);

        // After waking, poll should return data
        let result = queue.poll_action(&mut cx);
        if let std::task::Poll::Ready(crate::Action::Data(data)) = result {
            assert_eq!(data, 42);
        } else {
            panic!("Expected data after wake");
        }
    }
}
