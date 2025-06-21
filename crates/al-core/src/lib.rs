mod command;
mod event;
#[cfg(feature = "json")]
mod event_json;
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
    #[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventA;

    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
    struct TestEventB;

    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
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
            #[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
            #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
            pub struct DuplicateEvent;
        }

        mod crate_b {
            use super::*;

            #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
            #[cfg_attr(feature = "binary", derive(bincode::Encode, bincode::Decode))]
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
        todo!()
    }

    #[cfg(feature = "binary")]
    #[test]
    fn event_binary() {
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

        #[cfg(feature = "binary")]
        {
            fn has_binary_requirements<T: bincode::Encode + bincode::Decode<T>>() {}

            has_binary_requirements::<TestEventA>();
            has_binary_requirements::<TestEventB>();
            has_binary_requirements::<TestEventPayload>();
        }
    }
}
