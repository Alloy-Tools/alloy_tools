// map `self` to `al_events` allowing the use of derive macros that use `al_events::..`
extern crate self as al_events;
mod command;
mod context;
mod event;
mod markers;
mod message;
mod query;
#[cfg(feature = "serde")]
mod visitor;

#[cfg(feature = "serde")]
pub use command::{
    try_register_command, try_register_command_with, CommandDeserializer, CommandRegistry,
    COMMAND_REGISTRY,
};
pub use command::{Command, CommandHelpers, CommandMarker};
#[cfg(feature = "serde")]
pub use event::{
    try_register_event, try_register_event_with, EventDeserializer, EventRegistry, EVENT_REGISTRY,
};
pub use event::{Event, EventHelpers, EventMarker};
pub use markers::{MessageMarker, MessageRequirements, ObjectTraits};
use message::define_message_kind;
#[cfg(feature = "serde")]
pub use message::message_serde::{DeserializerFn, MessageDeserializer, MessageRegistry};
pub use message::{DynMessage, Message};
#[cfg(feature = "serde")]
pub use query::{
    try_register_query, try_register_query_with, QueryDeserializer, QueryRegistry, QUERY_REGISTRY,
};
pub use query::{Query, QueryHelpers, QueryMarker};

#[cfg(feature = "serde")]
pub use visitor::GenericMessageVisitor;

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn it_works() {}
}
