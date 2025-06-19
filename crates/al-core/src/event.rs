use crate::{command::Command, markers::EventMarker};
use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

pub trait Event: Send + Sync + Debug + Any + 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name(&self) -> &'static str;
    fn _clone_event(&self) -> Box<dyn Event>;
    fn _partial_equals_event(&self, other: &dyn Any) -> bool;
    fn _hash_event(&self, state: &mut dyn Hasher);

    fn to_cmd(self) -> Command
    where
        Self: Sized,
    {
        Command::Event(Box::new(self))
    }
}

// Blanket implementation for all compatible types
impl<T: EventMarker + Send + Sync + Clone + Default + PartialEq + Hash + Debug + Any + 'static>
    Event for T
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>() //Self
    }

    fn _clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn _partial_equals_event(&self, other: &dyn Any) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }

    fn _hash_event(&self, state: &mut dyn Hasher) {
        let mut wrapper = EventHasher(state);
        self.type_name().hash(&mut wrapper);
        self.hash(&mut wrapper);
    }
}

struct EventHasher<'a>(&'a mut dyn Hasher);

impl<'a> Hasher for EventHasher<'a> {
    fn finish(&self) -> u64 {
        self.0.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes)
    }
}
