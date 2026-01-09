#[cfg(feature = "event")]
use crate::event::Event;

/// A command that can be sent through the system to signal actions, including custom events.
#[cfg_attr(feature = "event", crate::event_requirements(Default))]
#[derive(Default)]
pub enum Command {
    #[cfg(feature = "event")]
    Event(Box<dyn Event>),
    Restart,
    Stop,
    #[default]
    Pulse,
}

#[cfg(feature = "event")]
impl Command {
    /// Returns true if the command is an event variant, otherwise false
    pub fn is_event(&self) -> bool {
        matches!(self, Command::Event(_))
    }

    /// Attempts to downcast the contained event to the specified event type, returning `None` if the command is not an event or if the downcast fails
    pub fn downcast_event<T: Event + crate::EventRequirements + 'static>(
        &self,
    ) -> Result<T, String> {
        match self {
            Command::Event(event) => crate::downcast_event(event),
            _ => Err("Command is not an Event variant".to_string()),
        }
    }

    /// Returns the type name of the contained event, returning `None` if the command is not an event variant
    pub fn event_type_name(&self) -> Option<String> {
        match self {
            Command::Event(event) => Some(event.as_ref().type_with_generics()),
            _ => None,
        }
    }
}
