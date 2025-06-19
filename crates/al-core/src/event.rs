use crate::{command::Command, markers::EventMarker};
use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// Used to wrap any passed hashers to support `Event::_hash_event()`
// ----------------------------------------------------------
struct EventHasher<'a>(&'a mut dyn Hasher);

impl<'a> Hasher for EventHasher<'a> {
    fn finish(&self) -> u64 {
        self.0.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes)
    }
}
// ----------------------------------------------------------

/// Used to mark other code with required traits as valid for `Event` support
// ----------------------------------------------------------
mod sealed {
    pub trait JsonFeature {}
    pub trait BinaryFeature {}
}

#[cfg(feature = "json")]
impl<T: serde::Serialize + for<'a> serde::Deserialize<'a>> sealed::JsonFeature for T {}
#[cfg(not(feature = "json"))]
impl<T> sealed::JsonFeature for T {}

#[cfg(feature = "binary")]
impl<T: bincode::Encode + bincode::Decode<T>> sealed::BinaryFeature for T {}
#[cfg(not(feature = "binary"))]
impl<T> sealed::BinaryFeature for T {}
// ----------------------------------------------------------

pub trait Event:
    Send + Sync + Debug + Any + sealed::JsonFeature + sealed::BinaryFeature + 'static
{
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
impl<
        T: EventMarker
            + 'static
            + Any
            + Send
            + Sync
            + Hash
            + Clone
            + Debug
            + Default
            + PartialEq
            + sealed::JsonFeature
            + sealed::BinaryFeature,
    > Event for T
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
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
