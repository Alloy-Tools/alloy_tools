use crate::{message::define_message_kind, DynMessage};

define_message_kind!(Event);

/// The `Event` trait defines the required methods for event types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Event: EventHelpers + erased_serde::Serialize {}
