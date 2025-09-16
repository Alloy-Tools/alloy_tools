use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type Registry<K, V> = HashMap<K, V>;

pub type SharedRegistry<K, V> = Arc<RwLock<Registry<K, V>>>;

// Event type name
type EventKey = &'static str;
// Event deserializer function
type EventDeserializer = Arc<
    dyn for<'de> Fn(
            &mut dyn erased_serde::Deserializer<'de>,
        ) -> Result<Box<dyn crate::Event>, erased_serde::Error>
        + Send
        + Sync,
>;

/// The EventRegistry is a registry for event deserializers.
/// It allows registering event types and retrieving their deserializers.
/// The key for each deserializer is the event type_name
pub struct EventRegistry {
    deserializers: SharedRegistry<EventKey, EventDeserializer>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            deserializers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_event<
        E: crate::Event + crate::EventMarker + for<'de> serde::de::Deserialize<'de> + 'static,
    >(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let type_name = E::_type_name();
        let deserializer: EventDeserializer =
            Arc::new(move |de: &mut dyn erased_serde::Deserializer<'_>| {
                let event: E = erased_serde::deserialize(de)?;
                Ok(Box::new(event))
            });

        self.deserializers
            .write()
            .map_err(|e| format!("Event registry write lock poisoned: {e}"))?
            .insert(type_name, deserializer);
        Ok(())
    }

    pub fn get_deserializer(
        &self,
        type_name: &str,
    ) -> Result<Option<EventDeserializer>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .deserializers
            .read()
            .map_err(|e| format!("Event registry read lock poisoned: {e}"))?
            .get(type_name)
            .cloned())
    }
}

#[macro_export]
macro_rules! register_event {
    ($registry:expr, $event_type:ty) => {{
        if let Err(e) = $registry.register_event::<$event_type>() {
            panic!(
                "Failed to register deserializer for event type {}: {}",
                stringify!($event_type),
                e
            );
        }
    }};
}
