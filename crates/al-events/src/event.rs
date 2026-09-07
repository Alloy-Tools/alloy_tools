use crate::{message::define_message_kind, DynMessage};

define_message_kind!(Event);

/// The `Event` trait defines the required methods for event types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Event: EventHelpers + erased_serde::Serialize {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};
    use al_derive::event;
    use al_structures::serde_utils::formats::JsonFormat;

    #[event]
    struct TestEvent {
        pub id: u32,
        pub name: String,
    }

    #[test]
    fn slice() {
        // Register the format
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();

        // Register the type
        let type_id = try_register_event::<TestEvent>().unwrap();

        // Create an instance and box it as a trait object
        let original = TestEvent {
            id: 42,
            name: "slice".to_string(),
        };
        let boxed = original.clone().to_msg();

        // Serialize to `(format_id, type_id, JSON)`
        let mut encoded = Vec::new();
        MESSAGE_FORMATS()
            .serialize(format_id, type_id, &boxed, &mut encoded)
            .unwrap();

        // Deserialize back to DynMessage::Command
        let decoded = MESSAGE_FORMATS()
            .deserialize_slice(MESSAGE_TYPE_REGISTRY(), &encoded)
            .unwrap();

        // Downcast to concrete type and compare
        let downcast = decoded
            .as_event()
            .and_then(|e| e.downcast_ref::<TestEvent>())
            .unwrap();
        assert_eq!(downcast, &original);
    }

    #[test]
    fn reader() {
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();
        let type_id = try_register_event::<TestEvent>().unwrap();
        let original = TestEvent {
            id: 42,
            name: "reader".to_string(),
        };

        let mut encoded = Vec::new();
        MESSAGE_FORMATS()
            .serialize(format_id, type_id, &original.clone().to_msg(), &mut encoded)
            .unwrap();

        let decoded = MESSAGE_FORMATS()
            .deserialize_reader(MESSAGE_TYPE_REGISTRY(), &mut encoded.as_slice())
            .unwrap();

        let downcast = decoded
            .as_event()
            .and_then(|e| e.downcast_ref::<TestEvent>())
            .unwrap();
        assert_eq!(downcast, &original);
    }
}
