//! # Transport implementations
//!
//! This module exposes runtime-agnostic transport implementations for the `Transport<T>`
//! state-machine core.
//!
//! ## Transport summary
//!
//! - [`Queue<T>`]
//!   - A lightweight in-memory FIFO queue transport.
//!   - Stores incoming items in a `VecDeque` and wakes a pending poll when new data arrives.
//!   - Best for simple single-threaded buffering and local pipeline composition.
//!
//! - [`BoundaryQueue<T>`]
//!   - A thread-safe queue for crossing sync/async boundaries.
//!   - Supports blocking receive, cancellable receive, and async receive via `Waker`.
//!   - Best when queue state needs to be shared safely across threads and async tasks.
//!
//! - [`Filter<T, F>`]
//!   - A transport adapter that forwards only items matching a predicate.
//!   - Incoming items that do not pass the predicate are discarded.
//!   - Useful for stream shaping and selective delivery.
//!
//! - [`Map<T, F>`]
//!   - A transport adapter that transforms each incoming item before buffering.
//!   - Supports type conversion or value transformation within the transport chain.
//!   - Useful for adapting incompatible item types or mapping data before delivery.
//!
//! ## Usage
//!
//! ```ignore
//! use al_transport::transports::{BoundaryQueue, Filter, Map, Queue};
//!
//! let mut queue = Queue::<i32>::new();
//! let boundary = BoundaryQueue::new();
//! let filter = Filter::new(|x: &i32| *x % 2 == 0);
//! let map = Map::new(|x: i32| x * 2);
//!
//! queue.handle_incoming(1);
//! queue.handle_incoming(2);
//!```
//!
//! The public API is exposed through this module, so docs for the re-exported transports
//! should be kept here rather than only inside child modules.
//!
//! ## When to use each transport
//!
//! - Use [`Queue<T>`] for the simplest FIFO transport when all activity stays within a single
//!   execution context.
//! - Use [`BoundaryQueue<T>`] when you need safe sharing across threads and async tasks.
//! - Use [`Filter<T, F>`] to filter incoming data before it is delivered.
//! - Use [`Map<T, F>`] to transform items while preserving transport semantics.

mod boundary_queue;
mod filter;
mod map;
mod queue;

pub use boundary_queue::{BoundaryQueue, BoundaryQueueError, BoundaryQueueTransport};
pub use filter::Filter;
pub use map::Map;
pub use queue::Queue;
