use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};

use crate::{Event, EventMarker};

type EventDeserializer = Arc<
    dyn for<'de> Fn(
            &mut dyn erased_serde::Deserializer<'de>,
        ) -> Result<Box<dyn Event>, erased_serde::Error>
        + Send
        + Sync,
>;

/// The EventRegistry is a registry for event deserializers.
/// It allows registering event types and retrieving their deserializers.
/// The key for each deserializer is a tuple of (the format, the event type name) with "json", "bincode", ect for format.
pub struct EventRegistry {
    deserializers: Mutex<HashMap<(&'static str, &'static str), EventDeserializer>>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            deserializers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_event<
        E: Event + EventMarker + for<'de> serde::de::Deserialize<'de> + 'static,
    >(
        &self,
        format: &'static str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let type_name = E::_type_name();
        let deserializer: EventDeserializer =
            Arc::new(move |de: &mut dyn erased_serde::Deserializer<'_>| {
                let event: E = erased_serde::deserialize(de)?;
                Ok(Box::new(event))
            });

        self.deserializers
            .lock()
            .map_err(|e| format!("Event registry mutex poisoned: {e}"))?
            .insert((format, type_name), deserializer);
        Ok(())
    }

    pub fn get_deserializer(
        &self,
        format: &str,
        type_name: &str,
    ) -> Result<Option<EventDeserializer>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .deserializers
            .lock()
            .map_err(|e| format!("Event registry mutex poisoned: {e}"))?
            .get(&(format, type_name))
            .cloned())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        &self,
        format: &'static str,
        deserializer: D,
    ) -> Result<Box<dyn Event>, D::Error> {
        deserializer.deserialize_map(crate::event_visitors::EventWrapper {
            registry: self,
            format,
        })
    }
}

#[macro_export]
macro_rules! register_event_type {
    ($registry:expr, $event_type:ty, $format:expr) => {{
        if let Err(e) = $registry.register_event::<$event_type>($format){
            panic!("Failed to register {} format for event type {}: {}", stringify!($format), stringify!($event_type), e);
        }
    }};
}
