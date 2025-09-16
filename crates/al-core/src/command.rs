use crate::event::Event;
use std::hash::Hash;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Default, PartialEq, Hash, std::fmt::Debug)]
pub enum Command {
    //#[serde(skip_deserializing)] // Skip deserialization as the `EventRegistry` handles it
    Event(Box<dyn Event>),
    Restart,
    Stop,
    #[default]
    Pulse,
}

impl Command {
    /// Returns true if the command is an event variant, otherwise false
    pub fn is_event(&self) -> bool {
        matches!(self, Command::Event(_))
    }

    /// Attempts to downcast the contained event to the specified event type, returning `None` if the command is not an event or if the downcast fails
    pub fn downcast_event<T: Event + 'static>(&self) -> Option<&T> {
        match self {
            Command::Event(event) => event.as_ref().as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Returns the type name of the contained event if the command is an event variant, otherwise returns `None`
    pub fn event_type_name(&self) -> Option<&'static str> {
        match self {
            Command::Event(event) => Some(event.as_ref().type_name()),
            _ => None,
        }
    }
}
