//! # Map Transport
//!
//! A transport that transforms items using a closure before queueing them.
//!
//! [`Map<T, F>`] takes incoming items of type `T`, applies a transformation function `F`,
//! and queues the transformed results. Items are buffered in a [`VecDeque`] until polled.
//!
//! ## Features
//!
//! - **Item transformation**: Applies a closure to each incoming item
//! - **Type conversion**: Can transform between different types
//! - **Buffering**: Transformed items are queued until polled
//! - **Status reporting**: Reports the number of buffered items
//!
//! ## Examples
//!
//! ```ignore
//! use al_transport::Map;
//! use al_transport::Transport;
//!
//! let mut map = Map::new(|x: i32| x * 2);  // Double each number
//!
//! map.handle_incoming(5).unwrap();   // Queues 10
//! map.handle_incoming(10).unwrap();  // Queues 20
//!
//! match map.poll() {
//!     std::task::Poll::Ready(Some(value)) => assert_eq!(value, 10),
//!     _ => panic!("Expected value"),
//! }
//! ```
//!
//! ## Panic Safety
//!
//! ⚠️ **If the transformation closure panics**, the item is lost and will not be queued.
//! Design your closure to handle unexpected inputs gracefully or wrap it with error handling
//! that returns a safe fallback value instead of panicking.
//!
//! ## Performance Notes
//!
//! - Closure invoked once per incoming item
//! - Useful for adapting between incompatible transport types
//! - Consider batching for compute-heavy transformations

use std::{collections::VecDeque, task::Waker};

use crate::{Transport, TransportItemRequirements};

pub struct Map<T, F> {
    f: F,
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T, F> Map<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + 'static> From<Map<T, F>>
    for Box<dyn Transport<T>>
{
    fn from(value: Map<T, F>) -> Self {
        Box::new(value)
    }
}

impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + 'static> Transport<T> for Map<T, F> {
    /// Receive incoming data and apply the transformation function.
    ///
    /// # Panics
    ///
    /// If the transformation function `f` panics, the calling code will panic and the data item
    /// will be lost. Ensure your transformation function is panic-free, or catch panics at a
    /// higher level if transformations may fail.
    fn handle_incoming(&mut self, data: T) {
        self.queue.push_back((self.f)(data));
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
        format!("Map Buffer Length: {}", self.queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_structures::noop_waker::noop_waker;

    #[test]
    fn map_transforms_data() {
        let mut map = Map::new(|x: i32| x * 2);

        map.handle_incoming(5);
        map.handle_incoming(10);
        map.handle_incoming(15);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, 10); // 5 * 2
        } else {
            panic!("Expected transformed data");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, 20); // 10 * 2
        } else {
            panic!("Expected transformed data");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, 30); // 15 * 2
        } else {
            panic!("Expected transformed data");
        }
    }

    #[test]
    fn map_with_string_transformation() {
        let mut map = Map::new(|s: String| s.to_uppercase());

        map.handle_incoming("hello".to_string());
        map.handle_incoming("world".to_string());

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, "HELLO");
        } else {
            panic!("Expected transformed string");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, "WORLD");
        } else {
            panic!("Expected transformed string");
        }
    }

    #[test]
    fn map_preserves_order() {
        let mut map = Map::new(|x: i32| x + 1);

        for i in 1..=5 {
            map.handle_incoming(i);
        }

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        for i in 1..=5 {
            if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
                assert_eq!(data, i + 1, "Items should be in order");
            } else {
                panic!("Expected data");
            }
        }
    }

    #[test]
    fn map_empty_returns_pending() {
        let mut map = Map::new(|x: i32| x * 2);

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        let result = map.poll_action(&mut cx);
        if let std::task::Poll::Pending = result {
            // Expected
        } else {
            panic!("Expected Pending on empty map");
        }
    }

    #[test]
    fn map_status_reports_buffer_length() {
        let mut map = Map::new(|x: i32| x * 2);

        assert_eq!(map.status(), "Map Buffer Length: 0");

        map.handle_incoming(1);
        map.handle_incoming(2);
        map.handle_incoming(3);

        assert_eq!(map.status(), "Map Buffer Length: 3");

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);
        let _ = map.poll_action(&mut cx);

        assert_eq!(map.status(), "Map Buffer Length: 2");
    }

    #[test]
    fn map_complex_transformation() {
        #[derive(Debug, Clone, PartialEq)]
        struct Point(i32, i32);

        let mut map = Map::new(|p: Point| Point(p.0 * 2, p.1 * 2));

        map.handle_incoming(Point(1, 2));
        map.handle_incoming(Point(3, 4));

        let waker = noop_waker().clone();
        let mut cx = std::task::Context::from_waker(&waker);

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, Point(2, 4));
        } else {
            panic!("Expected transformed Point");
        }

        if let std::task::Poll::Ready(crate::Action::Data(data)) = map.poll_action(&mut cx) {
            assert_eq!(data, Point(6, 8));
        } else {
            panic!("Expected transformed Point");
        }
    }
}
