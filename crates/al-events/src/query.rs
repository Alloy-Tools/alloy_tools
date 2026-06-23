use crate::{message::define_message_kind, DynMessage};

define_message_kind!(Query);

/// The `Query` trait defines the required methods for query types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Query: QueryHelpers + erased_serde::Serialize {
    //fn execute(self: Box<Self>); //TODO: add context parameter?
}
