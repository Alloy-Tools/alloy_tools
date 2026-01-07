use crate::SharedRegistry;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Type alias for the event deserializer function.
type EventDeserializer = Arc<
    dyn for<'de> Fn(
            &mut dyn erased_serde::Deserializer<'de>,
        ) -> Result<Box<dyn crate::Event>, erased_serde::Error>
        + Send
        + Sync,
>;

/// The EventRegistry is a registry for event deserializers.
/// It allows registering event types and retrieving their deserializers.
/// The key for each deserializer is the events type_name()
pub struct EventRegistry {
    deserializers: SharedRegistry<String, EventDeserializer>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            deserializers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers an event type with its deserializer function using erased_serde.
    pub fn register_event<
        E: crate::Event + crate::EventMarker + for<'de> serde::de::Deserialize<'de> + 'static,
    >(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deserializer: EventDeserializer =
            Arc::new(move |de: &mut dyn erased_serde::Deserializer<'_>| {
                let event: E = erased_serde::deserialize(de)?;
                Ok(Box::new(event))
            });

        self.deserializers
            .write()
            .map_err(|e| format!("Event serde registry write lock poisoned: {e}"))?
            .insert(
                <E as crate::EventMarker>::type_with_generics(),
                deserializer,
            );
        Ok(())
    }

    /// Returns a deserializer function for the given event type name if registered, None if not registered, or an error if the lock is poisoned.
    pub fn get_deserializer<T: AsRef<str>>(
        &self,
        type_name: T,
    ) -> Result<Option<EventDeserializer>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .deserializers
            .read()
            .map_err(|e| format!("Event serde registry read lock poisoned: {e}"))?
            .get(type_name.as_ref())
            .cloned())
    }
}

/// Macro to register an event type with the global event registry.
#[macro_export]
macro_rules! register_event {
    ($event:ty) => {{
        if let Err(e) = $crate::EVENT_REGISTRY.register_event::<$event>() {
            panic!(
                "Failed to register deserializer for event type {}: {}",
                stringify!($event),
                e
            );
        }
    }};
}

/// Macro to register an event type with any registry.
#[macro_export]
macro_rules! register_event_with {
    ($registry:expr, $event:ty) => {{
        if let Err(e) = $registry.register_event::<$event>() {
            panic!(
                "Failed to register deserializer for event type {} in registry {}: {}",
                stringify!($event),
                stringify!($registry),
                e
            );
        }
    }};
}
