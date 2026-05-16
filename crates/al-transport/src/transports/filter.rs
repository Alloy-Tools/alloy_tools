//! # Filter Transport
//!
//! A transport that filters items based on a predicate function.
//!
//! [`Filter<T, F>`] wraps another source of items and applies a predicate function to each one,
//! only queuing items that pass the test. Items are buffered in a [`VecDeque`] until polled.
//!
//! ## Features
//!
//! - **Selective forwarding**: Only items matching the predicate are queued
//! - **Buffering**: Items are stored until polled, supporting high-throughput filtering
//! - **Status reporting**: Reports the number of buffered items
//!
//! ## Examples
//!
//! ```ignore
//! use al_transport::Filter;
//! use al_transport::Transport;
//!
//! let mut filter = Filter::new(|x: &i32| x % 2 == 0);  // Only even numbers
//!
//! filter.handle_incoming(2).unwrap();  // Passes, queued
//! filter.handle_incoming(3).unwrap();  // Rejected, not queued
//! filter.handle_incoming(4).unwrap();  // Passes, queued
//!
//! // Poll to get even numbers only
//! ```
//!
//! ## Panic Safety
//!
//! ⚠️ **If the predicate function panics**, the item is lost and will not be queued.
//! Ensure your predicate is robust against unexpected inputs or consider wrapping it
//! with error handling that returns `false` instead of panicking.
//!
//! ## Performance Notes
//!
//! - Linear scan of predicate for each incoming item
//! - Rejected items are discarded immediately (no overhead)
//! - Useful for high-volume streams with strict filtering requirements

use std::{collections::VecDeque, task::Waker};

use crate::{Transport, TransportItemRequirements};

pub struct Filter<T, F> {
    predicate: F,
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T, F> Filter<T, F> {
    pub fn new(predicate: F) -> Self {
        Self {
            predicate,
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements, F: Fn(&T) -> bool + Send + 'static> Transport<T>
    for Filter<T, F>
{
    /// Receive incoming data and filter it based on the predicate function.
    ///
    /// # Panics
    ///
    /// If the predicate function panics, the calling code will panic and the data item will be lost.
    /// Ensure your predicate is panic-free, or catch panics at a higher level if filtering may fail.
    fn handle_incoming(&mut self, data: T) {
        if (self.predicate)(&data) {
            self.queue.push_back(data);
            if let Some(w) = self.waker.take() {
                w.wake();
            }
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
        format!("Filter Buffer Length: {}", self.queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_passes_matching_items() {
        let mut filter = Filter::new(|x: &i32| x > &5);

        filter.handle_incoming(3); // Filtered out
        filter.handle_incoming(7); // Passes
        filter.handle_incoming(2); // Filtered out
        filter.handle_incoming(10); // Passes

        let waker = crate::noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, 7);
        } else {
            panic!("Expected data 7");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, 10);
        } else {
            panic!("Expected data 10");
        }
    }

    #[test]
    fn filter_rejects_non_matching_items() {
        let mut filter = Filter::new(|x: &i32| x % 2 == 0); // Only even numbers

        filter.handle_incoming(1);
        filter.handle_incoming(2);
        filter.handle_incoming(3);
        filter.handle_incoming(4);

        let waker = crate::noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, 2);
        } else {
            panic!("Expected data 2");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, 4);
        } else {
            panic!("Expected data 4");
        }
    }

    #[test]
    fn filter_all_rejected_returns_pending() {
        let mut filter = Filter::new(|x: &i32| x > &100);

        filter.handle_incoming(1);
        filter.handle_incoming(2);
        filter.handle_incoming(3);

        let waker = crate::noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        let result = filter.poll_action(&mut cx);
        if let std::task::Poll::Pending = result {
            // Expected - all items filtered out
        } else {
            panic!("Expected Pending when all items filtered out");
        }
    }

    #[test]
    fn filter_status_reports_buffer_length() {
        let mut filter = Filter::new(|x: &i32| x > &5);

        filter.handle_incoming(1);
        filter.handle_incoming(10);
        filter.handle_incoming(20);

        assert_eq!(filter.status(), "Filter Buffer Length: 2");
    }

    #[test]
    fn filter_empty_then_pending() {
        let mut filter = Filter::new(|_: &i32| true);

        filter.handle_incoming(42);

        let waker = crate::noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        // Poll and consume the item
        let _ = filter.poll_action(&mut cx);

        // Next poll should be Pending
        let result = filter.poll_action(&mut cx);
        if let std::task::Poll::Pending = result {
            // Expected
        } else {
            panic!("Expected Pending on empty filter");
        }
    }

    #[test]
    fn filter_with_string_predicate() {
        let mut filter = Filter::new(|s: &String| s.len() > 3);

        filter.handle_incoming("hi".to_string()); // Filtered out
        filter.handle_incoming("hello".to_string()); // Passes
        filter.handle_incoming("ok".to_string()); // Filtered out
        filter.handle_incoming("rust".to_string()); // Passes

        let waker = crate::noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, "hello");
        } else {
            panic!("Expected 'hello'");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = filter.poll_action(&mut cx) {
            assert_eq!(data, "rust");
        } else {
            panic!("Expected 'rust'");
        }
    }
}
