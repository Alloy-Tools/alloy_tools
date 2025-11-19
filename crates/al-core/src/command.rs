use crate::event::Event;

/// A command that can be sent through the system to signal actions, including custom events.
#[crate::event_requirements]
pub enum Command {
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
    pub fn downcast_event<T: Event + crate::EventRequirements + 'static>(
        self,
    ) -> Result<T, String> {
        match self.try_downcast_event() {
            Ok(t) => Ok(t),
            Err(Ok(e)) => Err(format!(
                "Failed to downcast `dyn Event` of type '{}' to type '{}'",
                e.type_with_generics(),
                tynm::type_name::<T>()
            )),
            Err(Err(e)) => Err(e),
        }
    }

    pub fn try_downcast_event<T: Event + crate::EventRequirements + 'static>(
        self,
    ) -> Result<T, Result<Box<dyn Event>, String>> {
        match self {
            Command::Event(event) => match al_core::downcast_event::<T>(event) {
                Ok(t) => Ok(t),
                Err(event) => Err(Ok(event)),
            },
            _ => Err(Err("Command is not an Event variant".to_string())),
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
