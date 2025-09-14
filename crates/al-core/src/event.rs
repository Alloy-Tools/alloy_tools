use crate::{command::Command, markers::EventMarker};
use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// Used to mark other code with required traits as valid for serde support
mod sealed {
    use crate::EventMarker;

    #[cfg(not(feature = "serde"))]
    pub trait SerdeFeature {}

    #[cfg(feature = "serde")]
    pub trait SerdeFeature: erased_serde::Serialize {}

    #[cfg(feature = "serde")]
    impl<T: EventMarker + serde::Serialize> SerdeFeature for T {}
    #[cfg(not(feature = "serde"))]
    impl<T: EventMarker> SerdeFeature for T {}
}

pub fn downcast_event_box<T: Any>(b: Box<dyn Event>) -> Result<Box<T>, Box<dyn Any>> {
    let b = b as Box<dyn Any>;
    b.downcast::<T>()
}

pub trait Event:
    Send + Sync + Debug + Any + sealed::SerdeFeature + 'static
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
            + Send
            + Sync
            + Debug
            + Any
            + Hash
            + Clone
            + Default
            + PartialEq
            + sealed::SerdeFeature,
    > Event for T
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        T::_type_name()
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

    fn _hash_event(&self, mut state: &mut dyn Hasher) {
        self.type_name().hash(&mut state);
        self.hash(&mut state);
    }
}
