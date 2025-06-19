use crate::event::Event;
use std::hash::Hash;

#[derive(Clone, Default, PartialEq, Hash, std::fmt::Debug)]
pub enum Command {
    Event(Box<dyn Event>),
    Restart,
    Stop,
    #[default]
    Pulse,
}

impl Clone for Box<dyn Event> {
    fn clone(&self) -> Self {
        self._clone_event()
    }
}

impl PartialEq for Box<dyn Event> {
    fn eq(&self, other: &Self) -> bool {
        self._partial_equals_event(other.as_any())
    }
}

impl Hash for Box<dyn Event> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._hash_event(state);
    }
}

impl Command {
    pub fn is_event(&self) -> bool {
        matches!(self, Command::Event(_))
    }

    pub fn downcast_event<T: Event + 'static>(&self) -> Option<&T> {
        match self {
            Command::Event(event) => event.as_ref().as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    pub fn event_type_name(&self) -> Option<&'static str> {
        match self {
            Command::Event(event) => Some(event.as_ref().type_name()),
            _ => None,
        }
    }
}
