use crate::{message::define_message_kind, DynMessage};

define_message_kind!(Event);

#[cfg(feature = "serde")]
#[macro_export]
macro_rules! erase_event_factory {
    ($type:ty, $format_type:ty, $error_msg:expr) => {
        $crate::erase_message_factory!($type, Event, $format_type, $error_msg)
    };
}

/// The `Event` trait defines the required methods for event types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Event: EventHelpers + crate::markers::SerdeFeature {}

#[cfg(any(feature = "json", feature = "binary"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, FormatId, TypeId, MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};

    #[event]
    struct TestEvent {
        pub id: u32,
        pub name: String,
    }

    #[cfg(any(feature = "json", feature = "binary"))]
    fn test_slice_reader(format_id: FormatId, type_id: TypeId) {
        // Create an instance and box it as a trait object
        let original = TestEvent {
            id: 42,
            name: "serde".to_string(),
        };
        let boxed = original.clone().to_msg();
        let meta = boxed.meta();

        // Serialize to `(format_id, type_id, JSON)`
        let mut encoded = Vec::new();
        MESSAGE_FORMATS()
            .serialize_registered(
                MESSAGE_TYPE_REGISTRY(),
                format_id,
                type_id,
                &boxed,
                &mut encoded,
            )
            .unwrap();

        // Deserialize back to DynMessage
        let decoded_slice = MESSAGE_FORMATS()
            .deserialize_slice(MESSAGE_TYPE_REGISTRY(), &encoded)
            .unwrap();
        assert_eq!(meta, decoded_slice.meta());
        assert_eq!(
            &original,
            decoded_slice
                .as_event()
                .and_then(|(e, _)| e.downcast_ref::<TestEvent>())
                .unwrap()
        );

        // Deserialize back to DynMessage
        let decoded_reader = MESSAGE_FORMATS()
            .deserialize_reader(MESSAGE_TYPE_REGISTRY(), &mut encoded.as_slice())
            .unwrap();
        assert_eq!(meta, decoded_reader.meta());
        assert_eq!(
            &original,
            decoded_reader
                .as_event()
                .and_then(|(e, _)| e.downcast_ref::<TestEvent>())
                .unwrap()
        );
    }

    #[cfg(feature = "json")]
    #[test]
    fn serde_reoundtrip() {
        use al_structures::serde_utils::formats::JsonFormat;
        // Register the format
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();
        // Register the type
        let type_id = try_register_event::<TestEvent>().unwrap();
        test_slice_reader(format_id, type_id);
    }

    #[cfg(feature = "binary")]
    #[test]
    fn erased_roundtrip() {
        use crate::MESSAGE_TYPE_IDS;
        use al_structures::{
            collections::storage::RwLockStorage,
            serde_utils::{
                formats::BinaryFormat, serde_format::ErasedDeserialize,
                serde_registries::DirectFactory,
            },
        };
        use std::collections::HashMap;

        type BinaryInner = RwLockStorage<HashMap<TypeId, DirectFactory<DynMessage>>>;
        let type_factory = erase_event_factory!(
            TestEvent,
            BinaryFormat<DynMessage, BinaryInner>,
            "Failed to downcast to BinaryFormat"
        );
        let b_fmt = BinaryFormat::new(RwLockStorage::new(HashMap::new()));

        let type_id = b_fmt
            .register_with::<TestEvent, _, _, _>(MESSAGE_TYPE_IDS(), type_factory)
            .unwrap();
        let format_id = MESSAGE_FORMATS()
            .register_named("event-binary-test", b_fmt)
            .unwrap();

        test_slice_reader(format_id, type_id);
    }
}
