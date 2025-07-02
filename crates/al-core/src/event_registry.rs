use std::collections::HashMap;
use std::sync::Mutex;

use crate::Event;

type EventDeserializer = fn(&str) -> Box<dyn Event>;

lazy_static::lazy_static! {
    static ref EVENT_REGISTRY: Mutex<HashMap<&'static str, EventDeserializer>> = Mutex::new(HashMap::new());
}

pub fn register_event(type_name: &'static str, deserializer: EventDeserializer) {
    let mut deserializers = EVENT_REGISTRY
        .lock()
        .expect("Failed to lock the event registry mutex");
    deserializers.insert(type_name, deserializer);
}

#[derive(serde::Deserialize)]
struct EventWrapper {
    #[serde(rename = "type")]
    type_name: String,
    data: serde_json::Value,
}

pub fn deserialize_event(json: &str) -> Option<Box<dyn Event>> {
    let outer: serde_json::Value = serde_json::from_str(json).ok()?;
    let event_obj = outer.get("Event")?;
    let wrapper: EventWrapper = serde_json::from_value(event_obj.clone()).ok()?;
    let data_json = serde_json::to_string(&wrapper.data).ok()?;

    let registry = EVENT_REGISTRY
        .lock()
        .expect("Failed to lock the event registry mutex");
    registry
        .get(wrapper.type_name.as_str())
        .map(|deserializer| deserializer(&data_json))
}

#[macro_export]
macro_rules! register_event_type {
    ($event_type:ty) => {{
        fn deserialize(json: &str) -> Box<dyn Event> {
            let event: $event_type =
                serde_json::from_str(json).expect("Failed to deserialize event");
            Box::new(event)
        }
        $crate::event_registry::register_event(
            <$event_type as $crate::EventMarker>::_type_name(),
            deserialize,
        );
    }};
}
