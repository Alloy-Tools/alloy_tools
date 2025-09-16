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

/// Attempt to downcast a boxed event as dyn Any to a concrete type, returning the boxed concrete type on success and the boxed any on failure
pub fn downcast_event_box<T: Any>(b: Box<dyn Event>) -> Result<Box<T>, Box<dyn Any>> {
    let b = b as Box<dyn Any>;
    b.downcast::<T>()
}

pub trait Event: Send + Sync + Debug + Any + sealed::SerdeFeature + 'static {
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
    /// Returns a reference to self as a dyn Any
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Returns a mutable reference to self as a dyn Any
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Returns the type name of the event using the function from the `EventMarker` trait
    fn type_name(&self) -> &'static str {
        T::_type_name()
    }

    /// Clones the event and returns it as a boxed event
    fn _clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    /// Attempts to downcast the other event to the same type and compares them if successful, otherwise returns false
    fn _partial_equals_event(&self, other: &dyn Any) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }

    /// Hashes the type name and then the event itself into the given hasher
    fn _hash_event(&self, mut state: &mut dyn Hasher) {
        self.type_name().hash(&mut state);
        self.hash(&mut state);
    }
}

/// Implement Clone for Box<dyn Event> using the method defined in the Event trait
impl Clone for Box<dyn Event> {
    fn clone(&self) -> Self {
        self._clone_event()
    }
}

/// Implement PartialEq for Box<dyn Event> using the method defined in the Event trait
impl PartialEq for Box<dyn Event> {
    fn eq(&self, other: &Self) -> bool {
        self._partial_equals_event(other.as_any())
    }
}

/// Implement Hash for Box<dyn Event> using the method defined in the Event trait
impl Hash for Box<dyn Event> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._hash_event(state);
    }
}

/// Lazy static initialization of the global event registry
#[cfg(feature = "serde")]
pub static EVENT_REGISTRY: once_cell::sync::Lazy<crate::registry::EventRegistry> =
    once_cell::sync::Lazy::new(|| crate::registry::EventRegistry::new());

/// Implement serialization for `Box<dyn Event>` using the `SerWrap` struct and `erased_serde`
#[cfg(feature = "serde")]
impl serde::Serialize for Box<dyn Event> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        erased_serde::serialize(
            &(
                self.type_name().to_string(),
                self.as_ref() as &dyn erased_serde::Serialize,
            ),
            serializer,
        )
    }
}

/// Implement deserialization for `Box<dyn Event>` using the `EventVisitor` and the global event registry
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Box<dyn Event> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(crate::event_visitors::EventVisitor {
            registry: &EVENT_REGISTRY,
        })
    }
}
