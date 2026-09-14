use crate::{define_message_kind, DynMessage};

define_message_kind! { Command }

#[macro_export]
macro_rules! erase_command_factory {
    ($type:ty, $format_type:ty, $error_msg:expr) => {
        $crate::erase_message_factory!($type, Command, $format_type, $error_msg)
    };
}

/// The `Command` trait defines the required methods for command types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Command: CommandHelpers + crate::markers::SerdeFeature {}

#[cfg(feature = "serde")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command;
    #[cfg(any(feature = "json", feature = "binary"))]
    use crate::{FormatId, TypeId, MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};

    #[command]
    struct TestCommand {
        pub id: u32,
        pub name: String,
    }

    #[cfg(any(feature = "json", feature = "binary"))]
    fn test_slice_reader(format_id: FormatId, type_id: TypeId) {
        // Create an instance and box it as a trait object
        let original = TestCommand {
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
                .as_command()
                .and_then(|(c, _)| c.downcast_ref::<TestCommand>())
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
                .as_command()
                .and_then(|(c, _)| c.downcast_ref::<TestCommand>())
                .unwrap()
        );
    }

    #[cfg(feature = "binary")]
    fn make_binary_format(name: &str) -> (TypeId, FormatId) {
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
        let type_factory = erase_command_factory!(
            TestCommand,
            BinaryFormat<DynMessage, BinaryInner>,
            "Failed to downcast to BinaryFormat"
        );
        let b_fmt = BinaryFormat::new(RwLockStorage::new(HashMap::new()));

        let type_id = b_fmt
            .register_with::<TestCommand, _, _, _>(MESSAGE_TYPE_IDS(), type_factory)
            .unwrap();
        let format_id = MESSAGE_FORMATS().register_named(name, b_fmt).unwrap();
        (type_id, format_id)
    }

    #[cfg(feature = "json")]
    #[test]
    fn serde_roundtrip() {
        // Register the format
        let format_id = MESSAGE_FORMATS()
            .register(al_structures::serde_utils::formats::JsonFormat)
            .unwrap();
        // Register the type
        let type_id = try_register_command::<TestCommand>().unwrap();
        test_slice_reader(format_id, type_id);
    }

    #[cfg(feature = "binary")]
    #[test]
    fn erased_roundtrip() {
        let (type_id, format_id) = make_binary_format("command-binary-test-1");
        test_slice_reader(format_id, type_id);
    }

    #[test]
    fn idempotent_message_id() {
        #[command]
        struct ReRegisterCommand;
        let first = try_register_command::<ReRegisterCommand>().unwrap();
        let second = try_register_command::<ReRegisterCommand>().unwrap();
        assert_eq!(first, second);
        assert_eq!(second, ReRegisterCommand::type_message_id().unwrap())
    }

    #[test]
    fn concurrent_message_id() {
        use std::{thread, sync::{Arc, Barrier}};

        const THREADS: usize = 32;
        const ROUNDS: usize = 16;

        let barrier = Arc::new(Barrier::new(THREADS));

        let handles = (0..THREADS)
            .map(|_| {
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let mut observed = Vec::with_capacity(ROUNDS);
                    barrier.wait();
                    for round in 0..ROUNDS {
                        let from_register = try_register_command::<TestCommand>()
                            .unwrap_or_else(|e| panic!("round {round}: register failed: {e}"));
                        let from_lookup = TestCommand::type_message_id()
                            .unwrap_or_else(|e| panic!("round {round}: lookup failed: {e}"));
                        assert_eq!(
                            from_register, from_lookup,
                            "round {round}: register={from_register} lookup={from_lookup}"
                        );
                        observed.push(from_register);
                    }
                    observed
                })
            })
            .collect::<Vec<_>>();

        let mut all = Vec::with_capacity(THREADS * ROUNDS);
        for h in handles {
            all.extend(h.join().expect("Worker thread panicked"));
        }

        if !all.is_empty(){
            let first = all[0];
            if all.iter().any(|id| *id != first) {
                let mut distinct = all.clone();
                distinct.sort_unstable();
                distinct.dedup();
                panic!(
                    "registry returned different ids for the same type across threads: {distinct:?}"
                );
            }
        }
    }

    #[cfg(any(feature = "json", feature = "binary"))]
    #[test]
    fn to_format() {
        let (type_id, format_id) = cfg_select! {
            feature = "json" => {
                (
                    try_register_command::<TestCommand>().unwrap(),
                    MESSAGE_FORMATS().register(al_structures::serde_utils::formats::JsonFormat).unwrap(),
                )
            }
            feature = "binary" => {
                make_binary_format("command-binary-test-2")
            }
        };
        let msg = TestCommand {
            id: 42,
            name: "format".to_string(),
        }
        .to_msg();

        let mut data = Vec::new();
        msg.to_format(format_id, &mut data).unwrap();

        let mut data_with = Vec::new();
        msg.to_format_with(format_id, type_id, &mut data_with)
            .unwrap();
        assert_eq!(data, data_with);

        let decoded = MESSAGE_FORMATS()
            .deserialize_slice(MESSAGE_TYPE_REGISTRY(), &data)
            .unwrap();
        assert_eq!(msg, decoded);
    }
}
