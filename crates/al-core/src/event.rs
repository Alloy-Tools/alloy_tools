use crate::{command::Command, EventMarker, EventRequirements};
use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// Lazy static initialization of the global event registry allowing `Box<dyn Event>` and therefore `Command::Event` variants to be deserialized
#[cfg(feature = "serde")]
pub static EVENT_REGISTRY: once_cell::sync::Lazy<crate::serde_utils::registry::EventRegistry> =
    once_cell::sync::Lazy::new(|| crate::serde_utils::registry::EventRegistry::new());

/// Helper function to return the simple names of generic events
pub fn type_with_generics<T>(_: &T) -> String {
    tynm::type_name::<T>()
}

/// Helper function to downcast an event to a specific type, returning None if the downcast fails
pub fn downcast<T: Event + EventRequirements + 'static>(event: Box<dyn Event>) -> Result<T, Box<dyn Event>> {
    match event.as_any().downcast_ref::<T>() {
        Some(t) => Ok(t.clone()),
        None => Err(event),
    }
}

/// The `Event` trait defines the required methods for event types to exist in the system along with trait bounds that dont interfere with dyn usage
pub trait Event: Send + Sync + Debug + Any + crate::SerdeFeature + 'static {
    #[cfg(feature = "serde")]
    fn register(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        Self: for<'de> serde::Deserialize<'de>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_with_generics(&self) -> String;
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

/// Blanket implementation of the `Event` trait for all types that implement `EventMarker` and the required event traits
/// This allows any type that implements `EventMarker` and the traits required by `EventRequirements` to automatically be treated as an `Event`
impl<T: EventMarker + EventRequirements + crate::SerdeFeature> Event for T {
    #[cfg(feature = "serde")]
    fn register(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        Self: for<'de> serde::Deserialize<'de>,
    {
        crate::EVENT_REGISTRY.register_event(self)
    }

    /// Returns a reference to self as a dyn Any
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Returns a mutable reference to self as a dyn Any
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Returns the type name of the event with any generics simple names filled out using the `tynm` crate
    fn type_with_generics(&self) -> String {
        format!("{}::{}", T::_module_path(), type_with_generics(self))
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
        self.type_with_generics().hash(&mut state);
        self.hash(&mut state);
    }
}

/// Implement Clone for Box<dyn Event> allowing `Command::Event` variants to clone their internal event
impl Clone for Box<dyn Event> {
    fn clone(&self) -> Self {
        self._clone_event()
    }
}

/// Implement PartialEq for Box<dyn Event> allowing `Command::Event` variants to compare their internal events
impl PartialEq for Box<dyn Event> {
    fn eq(&self, other: &Self) -> bool {
        self._partial_equals_event(other.as_any())
    }
}

/// Implement Hash for Box<dyn Event> allowing `Command::Event` variants to hash their internal events
impl Hash for Box<dyn Event> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._hash_event(state);
    }
}

/// Implement serialization for `&dyn Event` using `erased_serde` by serializing into the tuple format `(type name, type data)`
#[cfg(feature = "serde")]
impl serde::Serialize for &dyn Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        erased_serde::serialize(
            &(
                self.type_with_generics(),
                *self as &dyn erased_serde::Serialize,
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
        deserializer.deserialize_tuple(
            2,
            crate::serde_utils::event_visitors::EventVisitor {
                registry: &EVENT_REGISTRY,
            },
        )
    }
}

/// Implement serialization for `Box<dyn Event>` using `erased_serde` by serializing into the tuple format `(type name, type data)`
#[cfg(feature = "serde")]
impl serde::Serialize for Box<dyn Event> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.as_ref() as &dyn Event).serialize(serializer)
    }
}
