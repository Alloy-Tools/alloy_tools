mod command;
mod event;
#[cfg(feature = "serde")]
mod event_visitors;
mod markers;
#[cfg(feature = "serde")]
mod registry;
#[cfg(feature = "serde")]
mod serde_format;

pub use command::Command;
pub use event::Event;
#[cfg(feature = "serde")]
pub use event::EVENT_REGISTRY;
pub use markers::EventMarker;
pub use markers::EventRequirements;
#[cfg(feature = "serde")]
pub use registry::Registry;
#[cfg(feature = "serde")]
pub use registry::SharedRegistry;
#[cfg(all(feature = "serde", feature = "binary"))]
pub use serde_format::BinarySerde;
#[cfg(all(feature = "serde", feature = "json"))]
pub use serde_format::JsonSerde;
#[cfg(feature = "serde")]
pub use serde_format::SerdeFormat;

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::event::Event;
    use crate::markers::EventMarker;
    use al_derive::EventMarker;
    use std::hash::{DefaultHasher, Hash, Hasher};

    /// Simple event for testing
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventA;

    /// Second simple event for testing
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventB;

    /// Event with payload for testing
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventPayload {
        value: u128,
        message: String,
    }
    const TEST_VAL: u128 = 7878;
    const TEST_MSG: &str = "Test";

    #[test]
    fn generics() {
        use crate::EventRequirements;

        #[allow(unused)]
        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct GenericEvent<T>(T);
        //TODO: add to generics test
        panic!("Test should do more than just compile");
    }

    /// Test converting event to command
    #[test]
    fn event_to_command() {
        let cmd = TestEventA.to_cmd();
        let payload_cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        assert!(matches!(cmd, Command::Event(_)));
        assert!(matches!(payload_cmd, Command::Event(_)));
    }

    /// Test downcasting commands back to their original event types
    #[test]
    fn verify_downcast() {
        let cmd = TestEventA.to_cmd();
        let payload_cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();

        assert!(cmd.downcast_event::<TestEventA>().is_some());
        assert!(cmd.downcast_event::<TestEventB>().is_none());
        assert!(Command::Stop.downcast_event::<TestEventB>().is_none());
        assert!(payload_cmd.downcast_event::<TestEventPayload>().is_some());
        assert!(payload_cmd.downcast_event::<TestEventA>().is_none());
    }

    /// Test function for identification of commands as events
    #[test]
    fn command_is_event() {
        let event_cmd = TestEventA.to_cmd();
        let restart_cmd = Command::Restart;
        let stop_cmd = Command::Stop;

        assert!(event_cmd.is_event());
        assert!(!restart_cmd.is_event());
        assert!(!stop_cmd.is_event());
    }

    /// Test function for retrieving event type names from commands
    #[test]
    fn event_type_identification() {
        let cmd_a = TestEventA.to_cmd();
        let cmd_b = TestEventB.to_cmd();

        assert_eq!(cmd_a.event_type_name(), Some(TestEventA.type_name()));
        assert_eq!(cmd_b.event_type_name(), Some(TestEventB.type_name()));
        assert!(Command::Stop.event_type_name().is_none());
    }

    /// Test to ensure event type names are unique across different crates to prevent registry collisions
    #[test]
    fn event_name_collisions() {
        mod crate_a {
            use super::*;

            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
            pub struct DuplicateEvent;
        }

        mod crate_b {
            use super::*;

            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
            pub struct DuplicateEvent;
        }

        assert_ne!(
            crate_a::DuplicateEvent.type_name(),
            crate_b::DuplicateEvent.type_name()
        );
    }

    /// Test hashing of events to ensure uniqueness based on type and content
    #[test]
    fn event_hash() {
        let event = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        };
        let event_diff_msg = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        };

        let mut hasher = DefaultHasher::new();
        event.hash(&mut hasher);
        let event_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        event_same.hash(&mut hasher);
        let event_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        event_diff_val.hash(&mut hasher);
        let event_diff_val_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        event_diff_msg.hash(&mut hasher);
        let event_diff_msg_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        TestEventA.hash(&mut hasher);
        let diff_event_hash = hasher.finish();

        assert_eq!(event_hash, event_same_hash);
        assert_ne!(event_hash, event_diff_val_hash);
        assert_ne!(event_hash, event_diff_msg_hash);
        assert_ne!(event_hash, diff_event_hash)
    }

    /// Test hashing of commands to ensure uniqueness based on contained event type and content
    #[test]
    fn command_hash() {
        let cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_msg = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        }
        .to_cmd();

        let mut hasher = DefaultHasher::new();
        cmd.hash(&mut hasher);
        let cmd_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        cmd_same.hash(&mut hasher);
        let cmd_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        cmd_diff_val.hash(&mut hasher);
        let cmd_diff_val_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        cmd_diff_msg.hash(&mut hasher);
        let cmd_diff_msg_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        Command::Stop.hash(&mut hasher);
        let cmd_stop_hash = hasher.finish();

        assert_eq!(cmd_hash, cmd_same_hash);
        assert_ne!(cmd_hash, cmd_diff_val_hash);
        assert_ne!(cmd_hash, cmd_diff_msg_hash);
        assert_ne!(cmd_hash, cmd_stop_hash)
    }

    /// Test cloning of events
    #[test]
    fn event_clone() {
        let original = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    /// Test cloning of commands
    #[test]
    fn command_clone() {
        let original = TestEventA.to_cmd();
        let cloned = original.clone();
        let original_payload = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cloned_payload = original_payload.clone();

        assert_eq!(original, cloned);
        assert_eq!(original_payload, cloned_payload);

        if let Some(original_event) = original_payload.downcast_event::<TestEventPayload>() {
            assert_eq!(original_event.value, TEST_VAL);
            assert_eq!(&original_event.message, TEST_MSG);
        } else {
            panic!("Downcast failed.");
        }
        if let Some(cloned_event) = cloned_payload.downcast_event::<TestEventPayload>() {
            assert_eq!(cloned_event.value, TEST_VAL);
            assert_eq!(&cloned_event.message, TEST_MSG);
        } else {
            panic!("Downcast failed.");
        }
    }

    /// Test partial equality of commands
    #[test]
    fn command_partial_equals() {
        let cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_str = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        }
        .to_cmd();

        assert_eq!(cmd, cmd.clone());
        assert_eq!(cmd, cmd_same);
        assert_ne!(cmd, cmd_diff_val);
        assert_ne!(cmd, cmd_diff_str);
        assert_ne!(cmd, TestEventA.to_cmd());
        assert_ne!(cmd, Command::Stop);
        assert_ne!(Command::Stop, Command::Pulse);
        assert_eq!(Command::Stop, Command::Stop);
    }

    /// Test to ensure events meet thread safety and trait requirements
    #[test]
    fn event_marker_thread_safety() {
        fn has_marker_requirements<
            T: Send + Sync + Clone + Default + PartialEq + std::fmt::Debug + std::any::Any + 'static,
        >() {
        }

        has_marker_requirements::<TestEventA>();
        has_marker_requirements::<TestEventB>();
        has_marker_requirements::<TestEventPayload>();

        #[cfg(feature = "serde")]
        {
            fn has_serde_requirements<T: serde::Serialize + for<'a> serde::Deserialize<'a>>() {}

            has_serde_requirements::<TestEventA>();
            has_serde_requirements::<TestEventB>();
            has_serde_requirements::<TestEventPayload>();
        }
    }

    /// Test serialization and deserialization of events using JSON format
    #[cfg(all(feature = "serde", feature = "json"))]
    #[test]
    fn event_json() {
        use crate::{JsonSerde, SerdeFormat};

        let event = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        };
        let event_diff_str = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        };

        let event_json = JsonSerde.serialize_event(&event).unwrap();
        let same_json = JsonSerde.serialize_event(&event_same).unwrap();
        let val_json = JsonSerde.serialize_event(&event_diff_val).unwrap();
        let str_json = JsonSerde.serialize_event(&event_diff_str).unwrap();
        let a_json = JsonSerde.serialize_event(&TestEventA {}).unwrap();

        assert_eq!(event_json, JsonSerde.serialize_event(&event).unwrap());
        assert_eq!(event_json, same_json);
        assert_ne!(event_json, val_json);
        assert_ne!(event_json, str_json);
        assert_ne!(event_json, a_json);

        let new_event: TestEventPayload = JsonSerde.deserialize_event(&event_json).unwrap();
        let new_same: TestEventPayload = JsonSerde.deserialize_event(&same_json).unwrap();
        let new_val: TestEventPayload = JsonSerde.deserialize_event(&val_json).unwrap();
        let new_str: TestEventPayload = JsonSerde.deserialize_event(&str_json).unwrap();
        let new_a: TestEventA = JsonSerde.deserialize_event(&a_json).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(TestEventA, new_a);
    }

    /// Test serialization and deserialization of commands using JSON format
    #[cfg(all(feature = "serde", feature = "json"))]
    #[test]
    fn command_json() {
        use crate::{register_event, JsonSerde, SerdeFormat};

        let cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_str = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        }
        .to_cmd();

        let cmd_json = JsonSerde.serialize_command(&cmd).unwrap();
        let same_json = JsonSerde.serialize_command(&cmd_same).unwrap();
        let val_json = JsonSerde.serialize_command(&cmd_diff_val).unwrap();
        let str_json = JsonSerde.serialize_command(&cmd_diff_str).unwrap();
        let a_json = JsonSerde.serialize_command(&TestEventA.to_cmd()).unwrap();
        let b_json = JsonSerde.serialize_command(&TestEventB.to_cmd()).unwrap();

        assert_eq!(cmd_json, JsonSerde.serialize_command(&cmd).unwrap());
        assert_eq!(cmd_json, same_json);
        assert_ne!(cmd_json, val_json);
        assert_ne!(cmd_json, str_json);
        assert_ne!(cmd_json, a_json);
        assert_ne!(a_json, b_json);

        register_event!(TestEventPayload);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_cmd: Command = JsonSerde.deserialize_command(&cmd_json).unwrap();
        let new_cmd_second: Command = JsonSerde.deserialize_command(&cmd_json).unwrap();
        let new_cmd_same: Command = JsonSerde.deserialize_command(&same_json).unwrap();
        let new_cmd_val: Command = JsonSerde.deserialize_command(&val_json).unwrap();
        let new_cmd_str: Command = JsonSerde.deserialize_command(&str_json).unwrap();
        let new_cmd_a: Command = JsonSerde.deserialize_command(&a_json).unwrap();
        let new_cmd_b: Command = JsonSerde.deserialize_command(&b_json).unwrap();

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(new_cmd_a, TestEventA.to_cmd());
        assert_eq!(new_cmd_b, TestEventB.to_cmd());

        let cmd_event = new_cmd.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_second = new_cmd_second.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_same = new_cmd_same.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_val = new_cmd_val.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_str = new_cmd_str.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_a = new_cmd_a.downcast_event::<TestEventA>().unwrap();
        let cmd_event_b = new_cmd_b.downcast_event::<TestEventB>().unwrap();

        assert_eq!(cmd_event, cmd_event_second);
        assert_eq!(cmd_event, cmd_event_same);
        assert_ne!(cmd_event, cmd_event_val);
        assert_ne!(cmd_event, cmd_event_str);
        assert_eq!(cmd_event_a, &TestEventA);
        assert_eq!(cmd_event_b, &TestEventB);
    }

    /// Test serialization and deserialization of events using binary format
    #[cfg(all(feature = "serde", feature = "binary"))]
    #[test]
    fn event_binary() {
        use crate::{BinarySerde, SerdeFormat};

        let event = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let event_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        };
        let event_diff_str = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        };

        let event_binary = BinarySerde.serialize_event(&event).unwrap();
        let same_binary = BinarySerde.serialize_event(&event_same).unwrap();
        let val_binary = BinarySerde.serialize_event(&event_diff_val).unwrap();
        let str_binary = BinarySerde.serialize_event(&event_diff_str).unwrap();
        let a_binary = BinarySerde.serialize_event(&TestEventA {}).unwrap();

        assert_eq!(event_binary, bitcode::serialize(&event).unwrap());
        assert_eq!(event_binary, same_binary);
        assert_ne!(event_binary, val_binary);
        assert_ne!(event_binary, str_binary);
        assert_ne!(event_binary, a_binary);

        let new_event: TestEventPayload = BinarySerde.deserialize_event(&event_binary).unwrap();
        let new_same: TestEventPayload = BinarySerde.deserialize_event(&same_binary).unwrap();
        let new_val: TestEventPayload = BinarySerde.deserialize_event(&val_binary).unwrap();
        let new_str: TestEventPayload = BinarySerde.deserialize_event(&str_binary).unwrap();
        let new_a: TestEventA = BinarySerde.deserialize_event(&a_binary).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(TestEventA, new_a);
    }

    /// Test serialization and deserialization of commands using binary format
    #[cfg(all(feature = "serde", feature = "binary"))]
    #[test]
    fn command_binary() {
        use crate::{register_event, BinarySerde, SerdeFormat};

        let cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_same = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_val = TestEventPayload {
            value: TEST_VAL + 1,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cmd_diff_str = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG[1..].to_string(),
        }
        .to_cmd();

        let cmd_binary = BinarySerde.serialize_command(&cmd).unwrap();
        let same_binary = BinarySerde.serialize_command(&cmd_same).unwrap();
        let val_binary = BinarySerde.serialize_command(&cmd_diff_val).unwrap();
        let str_binary = BinarySerde.serialize_command(&cmd_diff_str).unwrap();
        let a_binary = BinarySerde.serialize_command(&TestEventA.to_cmd()).unwrap();
        let b_binary = BinarySerde.serialize_command(&TestEventB.to_cmd()).unwrap();

        assert_eq!(cmd_binary, BinarySerde.serialize_command(&cmd).unwrap());
        assert_eq!(cmd_binary, same_binary);
        assert_ne!(cmd_binary, val_binary);
        assert_ne!(cmd_binary, str_binary);
        assert_ne!(cmd_binary, a_binary);
        assert_ne!(a_binary, b_binary);

        register_event!(TestEventPayload);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_cmd: Command = BinarySerde.deserialize_command(&cmd_binary).unwrap();
        let new_cmd_second: Command = BinarySerde.deserialize_command(&cmd_binary).unwrap();
        let new_cmd_same: Command = BinarySerde.deserialize_command(&same_binary).unwrap();
        let new_cmd_val: Command = BinarySerde.deserialize_command(&val_binary).unwrap();
        let new_cmd_str: Command = BinarySerde.deserialize_command(&str_binary).unwrap();
        let new_cmd_a: Command = BinarySerde.deserialize_command(&a_binary).unwrap();
        let new_cmd_b: Command = BinarySerde.deserialize_command(&b_binary).unwrap();

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(new_cmd_a, TestEventA.to_cmd());
        assert_eq!(new_cmd_b, TestEventB.to_cmd());

        let cmd_event = new_cmd.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_second = new_cmd_second.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_same = new_cmd_same.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_val = new_cmd_val.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_str = new_cmd_str.downcast_event::<TestEventPayload>().unwrap();
        let cmd_event_a = new_cmd_a.downcast_event::<TestEventA>().unwrap();
        let cmd_event_b = new_cmd_b.downcast_event::<TestEventB>().unwrap();

        assert_eq!(cmd_event, cmd_event_second);
        assert_eq!(cmd_event, cmd_event_same);
        assert_ne!(cmd_event, cmd_event_val);
        assert_ne!(cmd_event, cmd_event_str);
        assert_eq!(cmd_event_a, &TestEventA);
        assert_eq!(cmd_event_b, &TestEventB);
    }
}
