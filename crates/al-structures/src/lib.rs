//! Helper structures used by Alloy crates for cancellation, enum utilities, noop wakers, ect.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(any(feature = "cancellation", doc))]
pub mod cancellation;

#[cfg(any(feature = "enums", doc))]
pub mod enums;

#[cfg(any(feature = "traits", doc))]
pub mod traits;

#[cfg(any(feature = "noop_waker", doc))]
pub mod noop_waker;

#[cfg(any(feature = "race", doc))]
mod race;
#[cfg(any(feature = "race", doc))]
pub use race::Race;

#[cfg(any(feature = "collections", doc))]
pub mod collections;