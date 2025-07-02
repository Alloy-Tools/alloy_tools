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
    ) {
        let type_name = E::_type_name();
        let deserializer: EventDeserializer =
            Arc::new(move |de: &mut dyn erased_serde::Deserializer<'_>| {
                let event: E = erased_serde::deserialize(de)?;
                Ok(Box::new(event))
            });

        let mut deserializers = self
            .deserializers
            .lock()
            .expect("Failed to lock the event registry mutex");
        deserializers.insert((format, type_name), deserializer);
    }

    pub fn get_deserializer(&self, format: &str, type_name: &str) -> Option<EventDeserializer> {
        let deserializers = self
            .deserializers
            .lock()
            .expect("Failed to lock the event registry mutex");
        deserializers.get(&(format, type_name)).cloned()
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
        $registry.register_event::<$event_type>($format);
    }};
}
