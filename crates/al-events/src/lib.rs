// map `self` to `al_events` allowing the use of derive macros that use `al_events::..`
extern crate self as al_events;
mod command;
mod context;
mod event;
mod markers;
mod message;
mod query;

// Expose `TypeId` and FormatId here
#[cfg(feature = "serde")]
pub type TypeId = al_structures::serde_utils::serde_registries::TypeId;
#[cfg(feature = "serde")]
pub type FormatId = al_structures::serde_utils::serde_registries::FormatId;

#[cfg(feature = "serde")]
pub use command::{try_register_command, try_register_command_with};
pub use command::{Command, CommandHelpers, CommandMarker};
#[cfg(feature = "serde")]
pub use event::{try_register_event, try_register_event_with};
pub use event::{Event, EventHelpers, EventMarker};
pub use markers::{MessageMarker, MessageRequirements, ObjectTraits};
use message::define_message_kind;
#[cfg(feature = "borrow")]
pub use message::BorrowedMessage;
pub use message::{DynMessage, Message};
#[cfg(feature = "serde")]
pub use message::{MESSAGE_FORMATS, MESSAGE_TYPE_IDS, MESSAGE_TYPE_REGISTRY};
#[cfg(feature = "serde")]
pub use query::{try_register_query, try_register_query_with};
pub use query::{Query, QueryHelpers, QueryMarker};
