mod command;
mod event;
#[cfg(feature = "serde")]
mod event_registry;
#[cfg(feature = "serde")]
mod event_visitors;
mod markers;
#[cfg(feature = "serde")]
mod event_deserializer;

pub use command::Command;
pub use event::{downcast_event_box, Event};
#[cfg(feature = "serde")]
pub use event_deserializer::EventDeserializer;
#[cfg(feature = "serde")]
pub use event_registry::EventRegistry;
pub use markers::EventMarker;

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::event::Event;
    use crate::markers::EventMarker;
    use al_derive::EventMarker;
    use std::hash::{DefaultHasher, Hash, Hasher};

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventA;

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventB;

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventPayload {
        value: u128,
        message: String,
    }
    const TEST_VAL: u128 = 7878;
    const TEST_MSG: &str = "Test";

    #[test]
    fn event_to_command() {
        let cmd = TestEventA.to_cmd();
        assert!(cmd.is_event());
    }

    #[test]
    fn verify_downcast() {
        let cmd = TestEventA.to_cmd();

        assert!(cmd.downcast_event::<TestEventA>().is_some());
        assert!(cmd.downcast_event::<TestEventB>().is_none());
        assert!(Command::Stop.downcast_event::<TestEventB>().is_none());
    }

    #[test]
    fn check_types() {
        let event_cmd = TestEventA.to_cmd();
        let restart_cmd = Command::Restart;
        let stop_cmd = Command::Stop;

        assert!(event_cmd.is_event());
        assert!(!restart_cmd.is_event());
        assert!(!stop_cmd.is_event());
    }

    #[test]
    fn event_type_identification() {
        let cmd_a = TestEventA.to_cmd();
        let cmd_b = TestEventB.to_cmd();

        assert_eq!(cmd_a.event_type_name(), Some(TestEventA.type_name()));
        assert_eq!(cmd_b.event_type_name(), Some(TestEventB.type_name()));
        assert_eq!(Command::Stop.event_type_name(), None);
    }

    #[test]
    fn test_collision_protection() {
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

    #[test]
    fn command_clone() {
        let original = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let cloned = original.clone();

        assert_eq!(original, cloned);

        if let Some(original_event) = original.downcast_event::<TestEventPayload>() {
            assert_eq!(original_event.value, TEST_VAL);
            assert_eq!(&original_event.message, TEST_MSG);
        } else {
            panic!("Downcast failed.");
        }
        if let Some(cloned_event) = cloned.downcast_event::<TestEventPayload>() {
            assert_eq!(cloned_event.value, TEST_VAL);
            assert_eq!(&cloned_event.message, TEST_MSG);
        } else {
            panic!("Downcast failed.");
        }
    }

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

    #[cfg(feature = "test-utils")]
    #[test]
    fn event_serde_json() {
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

        let event_json = serde_json::to_string(&event).unwrap();
        let same_json = serde_json::to_string(&event_same).unwrap();
        let val_json = serde_json::to_string(&event_diff_val).unwrap();
        let str_json = serde_json::to_string(&event_diff_str).unwrap();
        let a_json = serde_json::to_string(&TestEventA {}).unwrap();

        assert_eq!(event_json, serde_json::to_string(&event).unwrap());
        assert_eq!(event_json, same_json);
        assert_ne!(event_json, val_json);
        assert_ne!(event_json, str_json);
        assert_ne!(event_json, a_json);
        //TODO: look into hooking any impl'd `EventMarker` serilize to serialize a wrapper with type_name, that way event type names can be inserted on event serialization layer rather than only command
        //assert_ne!(serde_json::to_string(&TestEventA{}).unwrap(), serde_json::to_string(&TestEventB{}).unwrap());

        let new_event: TestEventPayload = serde_json::from_str(&event_json).unwrap();
        let new_same: TestEventPayload = serde_json::from_str(&same_json).unwrap();
        let new_val: TestEventPayload = serde_json::from_str(&val_json).unwrap();
        let new_str: TestEventPayload = serde_json::from_str(&str_json).unwrap();
        let new_a: TestEventA = serde_json::from_str(&a_json).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(TestEventA, new_a);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn command_serde_json() {
        use crate::{downcast_event_box, register_event_type, EventRegistry};

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

        let cmd_json = serde_json::to_string(&cmd).unwrap();
        let same_json = serde_json::to_string(&cmd_same).unwrap();
        let val_json = serde_json::to_string(&cmd_diff_val).unwrap();
        let str_json = serde_json::to_string(&cmd_diff_str).unwrap();
        let a_json = serde_json::to_string(&TestEventA.to_cmd()).unwrap();
        let b_json = serde_json::to_string(&TestEventB.to_cmd()).unwrap();

        assert_eq!(cmd_json, serde_json::to_string(&cmd).unwrap());
        assert_eq!(cmd_json, same_json);
        assert_ne!(cmd_json, val_json);
        assert_ne!(cmd_json, str_json);
        assert_ne!(cmd_json, a_json);
        assert_ne!(a_json, b_json);

        let event_registry = EventRegistry::new();
        let json_format = "json";

        register_event_type!(event_registry, TestEventPayload, json_format);
        register_event_type!(event_registry, TestEventA, json_format);
        register_event_type!(event_registry, TestEventB, json_format);

        let new_cmd = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&cmd_json),
            )
            .unwrap();
        let new_cmd_second = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&cmd_json),
            )
            .unwrap();
        let new_cmd_same = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&same_json),
            )
            .unwrap();
        let new_cmd_val = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&val_json),
            )
            .unwrap();
        let new_cmd_str = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&str_json),
            )
            .unwrap();
        let new_cmd_a = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&a_json),
            )
            .unwrap();
        let new_cmd_b = event_registry
            .deserialize(
                json_format,
                &mut serde_json::Deserializer::from_str(&b_json),
            )
            .unwrap();

        let new_cmd = downcast_event_box::<TestEventPayload>(new_cmd)
            .unwrap()
            .to_cmd();
        let new_cmd_second = downcast_event_box::<TestEventPayload>(new_cmd_second)
            .unwrap()
            .to_cmd();
        let new_cmd_same = downcast_event_box::<TestEventPayload>(new_cmd_same)
            .unwrap()
            .to_cmd();
        let new_cmd_val = downcast_event_box::<TestEventPayload>(new_cmd_val)
            .unwrap()
            .to_cmd();
        let new_cmd_str = downcast_event_box::<TestEventPayload>(new_cmd_str)
            .unwrap()
            .to_cmd();

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(
            new_cmd_a
                .as_any()
                .downcast_ref::<TestEventA>()
                .unwrap()
                .clone()
                .to_cmd(),
            TestEventA.to_cmd()
        );
        assert_eq!(
            new_cmd_b
                .as_any()
                .downcast_ref::<TestEventB>()
                .unwrap()
                .clone()
                .to_cmd(),
            TestEventB.to_cmd()
        );
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn event_bitcode() {
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

        let event_binary = bitcode::serialize(&event).unwrap();
        let same_binary = bitcode::serialize(&event_same).unwrap();
        let val_binary = bitcode::serialize(&event_diff_val).unwrap();
        let str_binary = bitcode::serialize(&event_diff_str).unwrap();
        let a_binary = bitcode::serialize(&TestEventA {}).unwrap();

        assert_eq!(event_binary, bitcode::serialize(&event).unwrap());
        assert_eq!(event_binary, same_binary);
        assert_ne!(event_binary, val_binary);
        assert_ne!(event_binary, str_binary);
        assert_ne!(event_binary, a_binary);
        //TODO: same as event_serde_json
        //assert_ne!(bitcode::serialize(&TestEventA {}).unwrap(), bitcode::serialize(&TestEventB {}).unwrap());

        let new_event: TestEventPayload = bitcode::deserialize(&event_binary).unwrap();
        let new_same = bitcode::deserialize::<TestEventPayload>(&same_binary).unwrap();
        let new_val: TestEventPayload = bitcode::deserialize(&val_binary).unwrap();
        let new_str: TestEventPayload = bitcode::deserialize(&str_binary).unwrap();
        let new_a: TestEventA = bitcode::deserialize(&a_binary).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(TestEventA, new_a);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn command_bitcode() {
        use crate::{downcast_event_box, register_event_type, EventRegistry};

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

        let cmd_binary = bitcode::serialize(&cmd).unwrap();
        let same_binary = bitcode::serialize(&cmd_same).unwrap();
        let val_binary = bitcode::serialize(&cmd_diff_val).unwrap();
        let str_binary = bitcode::serialize(&cmd_diff_str).unwrap();
        let a_binary = bitcode::serialize(&TestEventA.to_cmd()).unwrap();
        let b_binary = bitcode::serialize(&TestEventB.to_cmd()).unwrap();

        println!("cmd_binary: {:?}", cmd_binary);
        println!("same_binary: {:?}", same_binary);
        println!("val_binary: {:?}", val_binary);
        println!("str_binary: {:?}", str_binary);
        println!("a_binary: {:?}", a_binary);
        println!("b_binary: {:?}", b_binary);

        assert_eq!(cmd_binary, bitcode::serialize(&cmd).unwrap());
        assert_eq!(cmd_binary, same_binary);
        assert_ne!(cmd_binary, val_binary);
        assert_ne!(cmd_binary, str_binary);
        assert_ne!(cmd_binary, a_binary);
        assert_ne!(a_binary, b_binary);

        let event_registry = EventRegistry::new();
        let binary_format = "binary";

        register_event_type!(event_registry, TestEventPayload, binary_format);
        register_event_type!(event_registry, TestEventA, binary_format);
        register_event_type!(event_registry, TestEventB, binary_format);

        let new_cmd = event_registry.deserialize(
            binary_format,
            &mut bitcode::Deserializer//bincode::Deserializer::from_slice(&cmd_binary, options),
        );
        //.unwrap();
        println!("new_cmd: {:?}", new_cmd);
        let new_cmd_second = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&cmd_binary, options),
            )
            .unwrap();
        let new_cmd_same = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&same_binary, options),
            )
            .unwrap();
        let new_cmd_val = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&val_binary, options),
            )
            .unwrap();
        let new_cmd_str = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&str_binary, options),
            )
            .unwrap();
        let new_cmd_a = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&a_binary, options),
            )
            .unwrap();
        let new_cmd_b = event_registry
            .deserialize(
                binary_format,
                &mut bincode::Deserializer::from_slice(&b_binary, options),
            )
            .unwrap();

        let new_cmd = downcast_event_box::<TestEventPayload>(new_cmd.unwrap())
            .unwrap()
            .to_cmd();
        let new_cmd_second = downcast_event_box::<TestEventPayload>(new_cmd_second)
            .unwrap()
            .to_cmd();
        let new_cmd_same = downcast_event_box::<TestEventPayload>(new_cmd_same)
            .unwrap()
            .to_cmd();
        let new_cmd_val = downcast_event_box::<TestEventPayload>(new_cmd_val)
            .unwrap()
            .to_cmd();
        let new_cmd_str = downcast_event_box::<TestEventPayload>(new_cmd_str)
            .unwrap()
            .to_cmd();

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(
            new_cmd_a
                .as_any()
                .downcast_ref::<TestEventA>()
                .unwrap()
                .clone()
                .to_cmd(),
            TestEventA.to_cmd()
        );
        assert_eq!(
            new_cmd_b
                .as_any()
                .downcast_ref::<TestEventB>()
                .unwrap()
                .clone()
                .to_cmd(),
            TestEventB.to_cmd()
        );*/
    }
}
