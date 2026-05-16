//! # Queue Transport
//!
//! A basic in-memory queue transport with waker notification support.
//!
//! [`Queue<T>`] is a simple FIFO buffer that implements the [`Transport<T>`] trait.
//! It stores items in a [`VecDeque`] and maintains a single waker for async notification
//! when the queue transitions from empty to non-empty.
//!
//! ## Features
//!
//! - **FIFO ordering**: Items are received in the order they were sent
//! - **Waker notification**: Async tasks waiting on empty queue are awakened when data arrives
//! - **Status reporting**: `status()` reports the current queue length
//!
//! ## Examples
//!
//! ```ignore
//! use al_transport::Queue;
//! use al_transport::Transport;
//!
//! let mut queue = Queue::<i32>::new();
//!
//! // Send items
//! queue.handle_incoming(42).unwrap();
//! queue.handle_incoming(99).unwrap();
//!
//! // Poll for items
//! match queue.poll() {
//!     std::task::Poll::Ready(Some(item)) => println!("Got: {}", item),
//!     _ => println!("Queue empty"),
//! }
//! ```
//!
//! ## Performance Notes
//!
//! - No allocations on send (items added to existing VecDeque)
//! - Waker stored once per empty queue; subsequent sends don't trigger additional work
//! - Perfect for in-process data flow with simple pub/sub patterns

mod boundary_queue;
mod filter;
mod map;
mod queue;

pub use boundary_queue::{BoundaryQueue, BoundaryQueueError, BoundaryQueueTransport};
pub use filter::Filter;
pub use map::Map;
pub use queue::Queue;
