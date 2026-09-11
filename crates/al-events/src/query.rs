use crate::{message::define_message_kind, DynMessage};

define_message_kind!(Query);

#[macro_export]
macro_rules! erase_query_factory {
    ($type:ty, $format_type:ty, $error_msg:expr) => {
        $crate::erase_message_factory!($type, Query, $format_type, $error_msg)
    };
}

/// The `Query` trait defines the required methods for query types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Query: QueryHelpers + erased_serde::Serialize {
    //TODO: add response type
    //type Response: Send + 'static;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{FormatId, TypeId, MESSAGE_FORMATS, MESSAGE_TYPE_IDS, MESSAGE_TYPE_REGISTRY};
    use al_derive::query;
    use al_structures::{
        collections::storage::RwLockStorage,
        serde_utils::{
            formats::{BinaryFormat, JsonFormat},
            serde_format::ErasedDeserialize,
            serde_registries::DirectFactory,
        },
        traits::DynTypeName,
    };

    #[query]
    struct TestQuery {
        pub id: u32,
        pub name: String,
    }

    fn test_slice_reader(format_id: FormatId, type_id: TypeId) {
        // Create an instance and box it as a trait object
        let original = TestQuery {
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
                .as_query()
                .and_then(|(q, _)| q.downcast_ref::<TestQuery>())
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
                .as_query()
                .and_then(|(q, _)| q.downcast_ref::<TestQuery>())
                .unwrap()
        );
    }

    #[test]
    fn serde_reoundtrip() {
        // Register the format
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();
        // Register the type
        let type_id = try_register_query::<TestQuery>().unwrap();
        test_slice_reader(format_id, type_id);
    }

    #[test]
    fn erased_roundtrip() {
        type BinaryInner = RwLockStorage<HashMap<TypeId, DirectFactory<DynMessage>>>;
        let type_factory = erase_query_factory!(
            TestQuery,
            BinaryFormat<DynMessage, BinaryInner>,
            "Failed to downcast to BinaryFormat"
        );
        let b_fmt = BinaryFormat::new(RwLockStorage::new(HashMap::new()));

        let type_id = b_fmt
            .register_with::<TestQuery, _, _, _>(MESSAGE_TYPE_IDS(), type_factory)
            .unwrap();
        let format_id = MESSAGE_FORMATS()
            .register_named(b_fmt.type_with_generics(), b_fmt)
            .unwrap();

        test_slice_reader(format_id, type_id);
    }
}
