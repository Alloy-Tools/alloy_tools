mod command;
mod event;
#[cfg(any(feature = "json", feature = "binary"))]
mod event_registry;
mod markers;

pub use command::Command;
pub use event::Event;
pub use markers::EventMarker;

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::event::Event;
    use crate::markers::EventMarker;
    use al_derive::EventMarker;
    use std::hash::{DefaultHasher, Hash, Hasher};

    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    //#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventA;

    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    //#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventB;

    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    //#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
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

            #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
            //#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
            #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
            pub struct DuplicateEvent;
        }

        mod crate_b {
            use super::*;

            #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
            //#[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
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

    #[cfg(feature = "json")]
    #[test]
    fn event_json() {
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

    #[cfg(feature = "json")]
    #[test]
    fn command_json() {
        use crate::{event_registry::deserialize_event, register_event_type};

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

        register_event_type!(TestEventPayload);
        register_event_type!(TestEventA);
        register_event_type!(TestEventB);

        let new_cmd = deserialize_event(&cmd_json).unwrap();
        let new_cmd_second = deserialize_event(&cmd_json).unwrap();
        let new_cmd_same = deserialize_event(&same_json).unwrap();
        let new_cmd_val = deserialize_event(&val_json).unwrap();
        let new_cmd_str = deserialize_event(&str_json).unwrap();
        let new_cmd_a = deserialize_event(&a_json).unwrap();
        let new_cmd_b = deserialize_event(&b_json).unwrap();

        //Recast to concrete types to verify equality
        let new_cmd = new_cmd.as_any().downcast_ref::<TestEventPayload>().unwrap();
        let new_cmd_second = new_cmd_second.as_any().downcast_ref::<TestEventPayload>().unwrap();
        let new_cmd_same = new_cmd_same.as_any().downcast_ref::<TestEventPayload>().unwrap();
        let new_cmd_val = new_cmd_val.as_any().downcast_ref::<TestEventPayload>().unwrap();
        let new_cmd_str = new_cmd_str.as_any().downcast_ref::<TestEventPayload>().unwrap();
        
        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert!(new_cmd_a.as_any().downcast_ref::<TestEventA>().is_some());
        assert!(new_cmd_b.as_any().downcast_ref::<TestEventB>().is_some());
    }

    #[cfg(feature = "binary")]
    #[test]
    fn event_binary() {
        todo!()
    }

    #[cfg(feature = "binary")]
    #[test]
    fn command_binary() {
        todo!()
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

        #[cfg(feature = "json")]
        {
            fn has_json_requirements<T: serde::Serialize + for<'a> serde::Deserialize<'a>>() {}

            has_json_requirements::<TestEventA>();
            has_json_requirements::<TestEventB>();
            has_json_requirements::<TestEventPayload>();
        }

        /*#[cfg(feature = "binary")]
        {
            fn has_binary_requirements<T: bincode::Encode + bincode::Decode<T>>() {}

            has_binary_requirements::<TestEventA>();
            has_binary_requirements::<TestEventB>();
            has_binary_requirements::<TestEventPayload>();
        }*/
    }
}
