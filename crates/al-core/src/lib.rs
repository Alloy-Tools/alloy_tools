// map `self` to `al_core` allowing the use of derive macros that use `al_core::..`
extern crate self as al_core;
#[cfg(feature = "command")]
mod command;
#[cfg(feature = "event")]
mod event;
mod markers;
#[cfg(feature = "serde")]
mod serde_utils;
#[cfg(feature = "task")]
mod task;
#[cfg(feature = "task")]
mod task_utils;
#[cfg(feature = "transport")]
mod transport;
#[cfg(feature = "transport")]
mod transports;
use std::marker::PhantomData;

#[cfg(feature = "command")]
pub use command::Command;
#[cfg(all(feature = "event", feature = "serde"))]
pub use event::EVENT_REGISTRY;
#[cfg(all(
    any(feature = "event", feature = "command"),
    feature = "serde",
    feature = "binary"
))]
pub use serde_utils::serde_format::BinarySerde;
#[cfg(all(
    any(feature = "event", feature = "command"),
    feature = "serde",
    feature = "json"
))]
pub use serde_utils::serde_format::JsonSerde;
#[cfg(all(any(feature = "event", feature = "command"), feature = "serde"))]
pub use serde_utils::serde_format::SerdeFormat;
#[cfg(feature = "event")]
pub use {
    al_derive::event, al_derive::event_requirements, al_derive::EventMarker as DeriveEventMarker,
    event::downcast as downcast_event, event::type_with_generics, event::Event,
    markers::EventMarker, markers::EventRequirements, markers::SerdeFeature,
};
#[cfg(feature = "transport")]
pub use {
    markers::NoOp, markers::TransportItemRequirements, markers::TransportRequirements,
    transport::Transport, transport::TransportError, transports::list::List,
    transports::publisher::Publisher, transports::queue::Queue,
    transports::transform::ApplyTransform, transports::transform::Transform,
    transports::transform::TransformFn,
};
#[cfg(feature = "task")]
pub use {
    markers::TaskStateRequirements, markers::TaskTypes, task::Task,
    task_utils::task_elements::TaskConfig, task_utils::task_elements::TaskError,
    task_utils::task_elements::TaskMode, task_utils::task_state::AsTaskState,
    task_utils::task_state::BaseTaskState, task_utils::task_state::ExtendedTaskState,
    task_utils::task_state::TaskState,
};
#[cfg(all(feature = "transport", feature = "task"))]
pub use {transports::buffered::Buffered, transports::link::Link, transports::splice::Splice};

/// A generic registry type using a HashMap.
pub type Registry<K, V> = std::collections::HashMap<K, V>;

/// A thread-safe shared registry using Arc and RwLock.
pub type SharedRegistry<K, V> = std::sync::Arc<std::sync::RwLock<Registry<K, V>>>;

/// Helper struct to debug strings without outer " displayed
pub struct DisplayString<S: AsRef<str>>(S);
impl<S: AsRef<str>> std::fmt::Debug for DisplayString<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_ref())
    }
}

/// Helper struct to concisely debug anything that can be coerced to a &[T] slice with N elements shown
struct SliceDebug<'a, T: 'a, V: AsRef<[T]>>(V, usize, PhantomData<&'a T>);
impl<'a, T: 'a + std::fmt::Debug, V: AsRef<[T]>> std::fmt::Debug for SliceDebug<'a, T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice = self.0.as_ref();
        let len = slice.len();
        let display_len = self.1.min(len);
        let mut debug_list = f.debug_list();
        for item in &slice[0..display_len] {
            debug_list.entry(item);
        }
        if len > display_len {
            debug_list.entry(&DisplayString(&format!("+{} more...", len - display_len)));
        }
        debug_list.finish()
    }
}
impl<'a, T: 'a, V: AsRef<[T]>> SliceDebug<'a, T, V> {
    const DEFAULT_MAX: usize = 3;

    pub fn new(slice: V) -> Self {
        Self(slice, Self::DEFAULT_MAX, PhantomData)
    }

    pub fn with_len(len: usize, slice: V) -> Self {
        Self(slice, len, PhantomData)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "command", feature = "event"))]
    use crate::Command;
    use crate::SliceDebug;
    #[cfg(feature = "event")]
    use crate::{event, Event, EventMarker};
    #[cfg(feature = "event")]
    use std::hash::{DefaultHasher, Hash, Hasher};

    #[cfg(feature = "event")]
    const TEST_VAL: u128 = 7878;

    #[cfg(feature = "event")]
    const TEST_MSG: &str = "Test";

    #[cfg(feature = "event")]
    /// Simple event for testing using the `event` attribute macro
    #[event]
    struct TestEventA;

    #[cfg(feature = "event")]
    /// Second simple event for testing using the `EventMarker` derive macro
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Default, PartialEq, Hash, Debug, al_derive::EventMarker)]
    struct TestEventB;

    #[cfg(feature = "event")]
    /// Enum for testing, along with omitting traits within the `event` attribute macro
    #[event(Default, Clone)]
    #[derive(Default, Clone)]
    enum TestEventEnum {
        #[default]
        A,
        B(u128),
        C(String),
    }

    #[cfg(feature = "event")]
    /// Event with payload for testing
    #[event]
    struct TestEventPayload {
        value: u128,
        message: String,
    }

    #[cfg(feature = "event")]
    /// Event with generic for testing
    #[event]
    struct TestEventGeneric<T>(T);

    #[test]
    fn slice_debug() {
        assert_eq!(
            format!("{:?}", SliceDebug::new([1, 2, 3, 4])),
            "[1, 2, 3, +1 more...]"
        );
        assert_eq!(
            format!("{:?}", SliceDebug::with_len(4, [1, 2, 3, 4])),
            "[1, 2, 3, 4]"
        );
        assert_eq!(
            format!("{:?}", SliceDebug::with_len(4, [1, 2, 3, 4, 5])),
            "[1, 2, 3, 4, +1 more...]"
        );
    }

    #[cfg(feature = "event")]
    #[test]
    fn type_with_generics() {
        assert_eq!(
            TestEventGeneric(String::from("")).type_with_generics(),
            "al_core::tests::TestEventGeneric<String>"
        );
        assert_eq!(
            <TestEventGeneric::<String> as EventMarker>::type_with_generics(),
            "al_core::tests::TestEventGeneric<String>"
        );
        assert_eq!(
            TestEventGeneric(0u8).type_with_generics(),
            "al_core::tests::TestEventGeneric<u8>"
        );
        assert_eq!(
            <TestEventGeneric::<u8> as EventMarker>::type_with_generics(),
            "al_core::tests::TestEventGeneric<u8>"
        );
        assert_eq!(
            TestEventGeneric(TestEventGeneric(String::from(""))).type_with_generics(),
            "al_core::tests::TestEventGeneric<TestEventGeneric<String>>"
        );
        assert_eq!(
            <TestEventGeneric::<TestEventGeneric::<String>> as EventMarker>::type_with_generics(),
            "al_core::tests::TestEventGeneric<TestEventGeneric<String>>"
        );
    }

    #[cfg(all(feature = "command", feature = "event"))]
    /// Test converting event to command
    #[test]
    fn event_to_command() {
        let cmd = TestEventA.to_cmd();
        let enum_cmd = TestEventEnum::A.to_cmd();
        let payload_cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        assert!(matches!(cmd, Command::Event(_)));
        assert!(matches!(enum_cmd, Command::Event(_)));
        assert!(matches!(payload_cmd, Command::Event(_)));
        assert!(matches!(generic_val, Command::Event(_)));
        assert!(matches!(generic_str, Command::Event(_)));
    }

    #[cfg(all(feature = "command", feature = "event"))]
    /// Test downcasting commands back to their original event types
    #[test]
    fn verify_downcast() {
        let cmd = TestEventA.to_cmd();
        let enum_a_cmd = TestEventEnum::A.to_cmd();
        let enum_b_cmd = TestEventEnum::B(1).to_cmd();
        let enum_c_cmd = TestEventEnum::C(TEST_MSG.to_string()).to_cmd();
        let payload_cmd = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();

        assert!(cmd.clone().downcast_event::<TestEventA>().is_ok());
        assert!(cmd.downcast_event::<TestEventB>().is_err());
        assert!(Command::Stop.downcast_event::<TestEventB>().is_err());
        assert!(enum_a_cmd.clone().downcast_event::<TestEventEnum>().is_ok());
        assert!(enum_a_cmd.downcast_event::<TestEventA>().is_err());
        assert!(enum_b_cmd.clone().downcast_event::<TestEventEnum>().is_ok());
        assert!(enum_b_cmd.downcast_event::<TestEventA>().is_err());
        assert!(enum_c_cmd.clone().downcast_event::<TestEventEnum>().is_ok());
        assert!(enum_c_cmd.downcast_event::<TestEventA>().is_err());
        assert!(payload_cmd
            .clone()
            .downcast_event::<TestEventPayload>()
            .is_ok());
        assert!(payload_cmd.downcast_event::<TestEventA>().is_err());
        assert!(generic_val
            .clone()
            .downcast_event::<TestEventGeneric<u128>>()
            .is_ok());
        assert!(generic_val
            .clone()
            .downcast_event::<TestEventGeneric<String>>()
            .is_err());
        assert!(generic_val.downcast_event::<TestEventA>().is_err());
        assert!(generic_str
            .clone()
            .downcast_event::<TestEventGeneric<String>>()
            .is_ok());
        assert!(generic_str
            .clone()
            .downcast_event::<TestEventGeneric<u128>>()
            .is_err());
        assert!(generic_str.downcast_event::<TestEventA>().is_err());
    }

    #[cfg(all(feature = "command", feature = "event"))]
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

    #[cfg(all(feature = "command", feature = "event"))]
    /// Test function for retrieving event type names from commands
    #[test]
    fn command_event_type_identification() {
        let cmd_a = TestEventA.to_cmd();
        let cmd_b = TestEventB.to_cmd();

        assert_eq!(
            cmd_a.event_type_name(),
            Some(TestEventA.type_with_generics())
        );
        assert_eq!(
            cmd_b.event_type_name(),
            Some(TestEventB.type_with_generics())
        );
        assert!(Command::Stop.event_type_name().is_none());
    }

    #[cfg(feature = "event")]
    /// Test to ensure event type names are unique across different crates to prevent registry collisions
    #[test]
    fn event_name_collisions() {
        mod crate_a {
            use super::*;

            #[event]
            pub struct DuplicateEvent;
        }

        mod crate_b {
            use super::*;

            #[event]
            pub struct DuplicateEvent;
        }

        assert_ne!(
            crate_a::DuplicateEvent.type_with_generics(),
            crate_b::DuplicateEvent.type_with_generics()
        );
    }

    #[cfg(feature = "event")]
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
        assert_ne!(event_hash, diff_event_hash);

        let generic_val = TestEventGeneric(TEST_VAL);
        let generic_val_same = TestEventGeneric(TEST_VAL);
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1);
        let generic_str = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string());

        let mut hasher = DefaultHasher::new();
        generic_val.hash(&mut hasher);
        let generic_val_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_val_same.hash(&mut hasher);
        let generic_val_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_val_diff.hash(&mut hasher);
        let generic_val_diff_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str.hash(&mut hasher);
        let generic_str_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str_same.hash(&mut hasher);
        let generic_str_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str_diff.hash(&mut hasher);
        let generic_str_diff_hash = hasher.finish();

        assert_eq!(generic_val_hash, generic_val_same_hash);
        assert_ne!(generic_val_hash, generic_val_diff_hash);
        assert_eq!(generic_str_hash, generic_str_same_hash);
        assert_ne!(generic_str_hash, generic_str_diff_hash);
    }

    #[cfg(all(feature = "command", feature = "event"))]
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
        assert_ne!(cmd_hash, cmd_stop_hash);

        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_same = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string()).to_cmd();

        let mut hasher = DefaultHasher::new();
        generic_val.hash(&mut hasher);
        let generic_val_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_val_same.hash(&mut hasher);
        let generic_val_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_val_diff.hash(&mut hasher);
        let generic_val_diff_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str.hash(&mut hasher);
        let generic_str_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str_same.hash(&mut hasher);
        let generic_str_same_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        generic_str_diff.hash(&mut hasher);
        let generic_str_diff_hash = hasher.finish();

        assert_eq!(generic_val_hash, generic_val_same_hash);
        assert_ne!(generic_val_hash, generic_val_diff_hash);
        assert_eq!(generic_str_hash, generic_str_same_hash);
        assert_ne!(generic_str_hash, generic_str_diff_hash);
    }

    #[cfg(feature = "event")]
    /// Test cloning of events
    #[test]
    fn event_clone() {
        let original = TestEventA;
        let cloned = original.clone();
        let enum_a_original = TestEventEnum::A;
        let enum_a_cloned = enum_a_original.clone();
        let enum_b_original = TestEventEnum::B(1);
        let enum_b_cloned = enum_b_original.clone();
        let enum_c_original = TestEventEnum::C(TEST_MSG.to_string());
        let enum_c_cloned = enum_c_original.clone();
        let payload_original = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        };
        let payload_cloned = payload_original.clone();
        let generic_val_original = TestEventGeneric(TEST_VAL);
        let generic_val_cloned = generic_val_original.clone();
        let generic_str_original = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_cloned = generic_str_original.clone();

        assert_eq!(original, cloned);
        assert_eq!(enum_a_original, enum_a_cloned);
        assert_eq!(enum_b_original, enum_b_cloned);
        assert_eq!(enum_c_original, enum_c_cloned);
        assert_eq!(payload_original, payload_cloned);
        assert_eq!(generic_val_original, generic_val_cloned);
        assert_eq!(generic_str_original, generic_str_cloned);
    }

    #[cfg(all(feature = "command", feature = "event"))]
    /// Test cloning of commands
    #[test]
    fn command_clone() {
        let original = TestEventA.to_cmd();
        let cloned = original.clone();
        let enum_original = TestEventEnum::A.to_cmd();
        let enum_cloned = enum_original.clone();
        let payload_original = TestEventPayload {
            value: TEST_VAL,
            message: TEST_MSG.to_string(),
        }
        .to_cmd();
        let payload_cloned = payload_original.clone();
        let generic_val_original = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_cloned = generic_val_original.clone();
        let generic_str_original = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_cloned = generic_str_original.clone();

        assert_eq!(original, cloned);
        assert_eq!(enum_original, enum_cloned);
        assert_eq!(payload_original, payload_cloned);
        assert_eq!(generic_val_original, generic_val_cloned);
        assert_eq!(generic_str_original, generic_str_cloned);

        match payload_original.downcast_event::<TestEventPayload>() {
            Ok(original_event) => {
                assert_eq!(original_event.value, TEST_VAL);
                assert_eq!(&original_event.message, TEST_MSG);
            }
            Err(e) => panic!("Downcast failed: {}", e),
        }
        match payload_cloned.downcast_event::<TestEventPayload>() {
            Ok(cloned_event) => {
                assert_eq!(cloned_event.value, TEST_VAL);
                assert_eq!(&cloned_event.message, TEST_MSG);
            }
            Err(e) => panic!("Downcast failed: {}", e),
        }

        match generic_val_original.downcast_event::<TestEventGeneric<u128>>() {
            Ok(original_event) => assert_eq!(original_event.0, TEST_VAL),
            Err(e) => panic!("Downcast failed: {}", e),
        }
        match generic_val_cloned.downcast_event::<TestEventGeneric<u128>>() {
            Ok(cloned_event) => assert_eq!(cloned_event.0, TEST_VAL),
            Err(e) => panic!("Downcast failed: {}", e),
        }

        match generic_str_original.downcast_event::<TestEventGeneric<String>>() {
            Ok(original_event) => assert_eq!(original_event.0, TEST_MSG),
            Err(e) => panic!("Downcast failed: {}", e),
        }
        match generic_str_cloned.downcast_event::<TestEventGeneric<String>>() {
            Ok(cloned_event) => assert_eq!(cloned_event.0, TEST_MSG),
            Err(e) => panic!("Downcast failed: {}", e),
        }
    }

    #[cfg(all(feature = "command", feature = "event"))]
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

        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_same = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string()).to_cmd();

        assert_eq!(cmd, cmd.clone());
        assert_eq!(cmd, cmd_same);
        assert_ne!(cmd, cmd_diff_val);
        assert_ne!(cmd, cmd_diff_str);
        assert_ne!(cmd, TestEventA.to_cmd());
        assert_ne!(cmd, Command::Stop);
        assert_eq!(generic_val, generic_val.clone());
        assert_eq!(generic_val, generic_val_same);
        assert_ne!(generic_val, generic_val_diff);
        assert_eq!(generic_str, generic_str.clone());
        assert_eq!(generic_str, generic_str_same);
        assert_ne!(generic_str, generic_str_diff);
        assert_ne!(Command::Stop, Command::Pulse);
        assert_eq!(Command::Stop, Command::Stop);
    }

    #[cfg(feature = "event")]
    /// Test to ensure events meet thread safety and trait requirements
    #[test]
    fn event_marker_thread_safety() {
        fn has_marker_requirements<
            T: Send + Sync + Clone + Default + PartialEq + std::fmt::Debug + std::any::Any + 'static,
        >() {
        }

        has_marker_requirements::<TestEventA>();
        has_marker_requirements::<TestEventB>();
        has_marker_requirements::<TestEventEnum>();
        has_marker_requirements::<TestEventPayload>();
        has_marker_requirements::<TestEventGeneric<u128>>();
        has_marker_requirements::<TestEventGeneric<String>>();

        #[cfg(feature = "serde")]
        {
            fn has_serde_requirements<T: serde::Serialize + for<'a> serde::Deserialize<'a>>() {}

            has_serde_requirements::<TestEventA>();
            has_serde_requirements::<TestEventB>();
            has_serde_requirements::<TestEventEnum>();
            has_serde_requirements::<TestEventPayload>();
            has_serde_requirements::<TestEventGeneric<u128>>();
            has_serde_requirements::<TestEventGeneric<String>>();
        }
    }

    /// Test serialization and deserialization of events using JSON format
    #[cfg(all(feature = "event", feature = "serde", feature = "json"))]
    #[test]
    fn event_json() {
        use crate::{register_event, JsonSerde, SerdeFormat};

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

        let generic_val = TestEventGeneric(TEST_VAL);
        let generic_val_same = TestEventGeneric(TEST_VAL);
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1);
        let generic_str = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string());

        let event_json = JsonSerde.serialize_event(&event).unwrap();
        let same_json = JsonSerde.serialize_event(&event_same).unwrap();
        let val_json = JsonSerde.serialize_event(&event_diff_val).unwrap();
        let str_json = JsonSerde.serialize_event(&event_diff_str).unwrap();
        let a_json = JsonSerde.serialize_event(&TestEventA).unwrap();
        let b_json = JsonSerde.serialize_event(&TestEventB).unwrap();

        assert_eq!(event_json, JsonSerde.serialize_event(&event).unwrap());
        assert_eq!(event_json, same_json);
        assert_ne!(event_json, val_json);
        assert_ne!(event_json, str_json);
        assert_ne!(event_json, a_json);
        assert_ne!(a_json, b_json);

        let enum_a_json = JsonSerde.serialize_event(&TestEventEnum::A).unwrap();
        let enum_b_json = JsonSerde
            .serialize_event(&TestEventEnum::B(TEST_VAL))
            .unwrap();
        let enum_b_diff_json = JsonSerde
            .serialize_event(&TestEventEnum::B(TEST_VAL + 1))
            .unwrap();
        let enum_c_json = JsonSerde
            .serialize_event(&TestEventEnum::C(TEST_MSG.to_string()))
            .unwrap();
        let enum_c_diff_json = JsonSerde
            .serialize_event(&TestEventEnum::C(TEST_MSG[1..].to_string()))
            .unwrap();

        assert_ne!(enum_a_json, a_json);
        assert_ne!(enum_a_json, enum_b_json);
        assert_ne!(enum_b_json, enum_b_diff_json);
        assert_ne!(enum_a_json, enum_c_json);
        assert_ne!(enum_c_json, enum_c_diff_json);
        assert_eq!(
            enum_a_json,
            JsonSerde.serialize_event(&TestEventEnum::A).unwrap()
        );

        let generic_val_json = JsonSerde.serialize_event(&generic_val).unwrap();
        let generic_val_same_json = JsonSerde.serialize_event(&generic_val_same).unwrap();
        let generic_val_diff_json = JsonSerde.serialize_event(&generic_val_diff).unwrap();
        let generic_str_json = JsonSerde.serialize_event(&generic_str).unwrap();
        let generic_str_same_json = JsonSerde.serialize_event(&generic_str_same).unwrap();
        let generic_str_diff_json = JsonSerde.serialize_event(&generic_str_diff).unwrap();

        assert_eq!(
            generic_val_json,
            JsonSerde.serialize_event(&generic_val).unwrap()
        );
        assert_eq!(generic_val_json, generic_val_same_json);
        assert_ne!(generic_val_json, generic_val_diff_json);
        assert_ne!(generic_val_json, a_json);
        assert_eq!(
            generic_str_json,
            JsonSerde.serialize_event(&generic_str).unwrap()
        );
        assert_eq!(generic_str_json, generic_str_same_json);
        assert_ne!(generic_str_json, generic_str_diff_json);
        assert_ne!(generic_str_json, a_json);

        register_event!(TestEventPayload);
        register_event!(TestEventGeneric<u128>);
        register_event!(TestEventGeneric<String>);
        register_event!(TestEventEnum);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_event: TestEventPayload = JsonSerde.deserialize_event(&event_json).unwrap();
        let new_same: TestEventPayload = JsonSerde.deserialize_event(&same_json).unwrap();
        let new_val: TestEventPayload = JsonSerde.deserialize_event(&val_json).unwrap();
        let new_str: TestEventPayload = JsonSerde.deserialize_event(&str_json).unwrap();
        let new_a: TestEventA = JsonSerde.deserialize_event(&a_json).unwrap();
        let new_b: TestEventB = JsonSerde.deserialize_event(&b_json).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(new_a, TestEventA);
        assert_eq!(new_b, TestEventB);

        let enum_new_a: TestEventEnum = JsonSerde.deserialize_event(&enum_a_json).unwrap();
        let enum_new_b: TestEventEnum = JsonSerde.deserialize_event(&enum_b_json).unwrap();
        let enum_new_b_diff: TestEventEnum =
            JsonSerde.deserialize_event(&enum_b_diff_json).unwrap();
        let enum_new_c: TestEventEnum = JsonSerde.deserialize_event(&enum_c_json).unwrap();
        let enum_new_c_diff: TestEventEnum =
            JsonSerde.deserialize_event(&enum_c_diff_json).unwrap();

        assert_eq!(enum_new_a, TestEventEnum::A);
        assert_eq!(enum_new_b, TestEventEnum::B(TEST_VAL));
        assert_eq!(enum_new_b_diff, TestEventEnum::B(TEST_VAL + 1));
        assert_eq!(enum_new_c, TestEventEnum::C(TEST_MSG.to_string()));
        assert_eq!(enum_new_c_diff, TestEventEnum::C(TEST_MSG[1..].to_string()));

        let new_generic_val: TestEventGeneric<u128> =
            JsonSerde.deserialize_event(&generic_val_json).unwrap();
        let new_generic_val_same: TestEventGeneric<u128> =
            JsonSerde.deserialize_event(&generic_val_same_json).unwrap();
        let new_generic_val_diff: TestEventGeneric<u128> =
            JsonSerde.deserialize_event(&generic_val_diff_json).unwrap();
        let new_generic_str: TestEventGeneric<String> =
            JsonSerde.deserialize_event(&generic_str_json).unwrap();
        let new_generic_str_same: TestEventGeneric<String> =
            JsonSerde.deserialize_event(&generic_str_same_json).unwrap();
        let new_generic_str_diff: TestEventGeneric<String> =
            JsonSerde.deserialize_event(&generic_str_diff_json).unwrap();

        assert_eq!(generic_val, new_generic_val);
        assert_eq!(generic_val_same, new_generic_val_same);
        assert_eq!(generic_val_diff, new_generic_val_diff);
        assert_eq!(generic_str, new_generic_str);
        assert_eq!(generic_str_same, new_generic_str_same);
        assert_eq!(generic_str_diff, new_generic_str_diff);
    }

    /// Test serialization and deserialization of commands using JSON format
    #[cfg(all(
        feature = "command",
        feature = "event",
        feature = "serde",
        feature = "json"
    ))]
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

        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_same = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string()).to_cmd();

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

        let enum_a_json = JsonSerde
            .serialize_command(&TestEventEnum::A.to_cmd())
            .unwrap();
        let enum_b_json = JsonSerde
            .serialize_command(&TestEventEnum::B(TEST_VAL).to_cmd())
            .unwrap();
        let enum_b_diff_json = JsonSerde
            .serialize_command(&TestEventEnum::B(TEST_VAL + 1).to_cmd())
            .unwrap();
        let enum_c_json = JsonSerde
            .serialize_command(&TestEventEnum::C(TEST_MSG.to_string()).to_cmd())
            .unwrap();
        let enum_c_diff_json = JsonSerde
            .serialize_command(&TestEventEnum::C(TEST_MSG[1..].to_string()).to_cmd())
            .unwrap();

        assert_ne!(enum_a_json, a_json);
        assert_ne!(enum_a_json, enum_b_json);
        assert_ne!(enum_b_json, enum_b_diff_json);
        assert_ne!(enum_a_json, enum_c_json);
        assert_ne!(enum_c_json, enum_c_diff_json);
        assert_eq!(
            enum_a_json,
            JsonSerde
                .serialize_command(&TestEventEnum::A.to_cmd())
                .unwrap()
        );

        let generic_val_json = JsonSerde.serialize_command(&generic_val).unwrap();
        let generic_val_same_json = JsonSerde.serialize_command(&generic_val_same).unwrap();
        let generic_val_diff_json = JsonSerde.serialize_command(&generic_val_diff).unwrap();
        let generic_str_json = JsonSerde.serialize_command(&generic_str).unwrap();
        let generic_str_same_json = JsonSerde.serialize_command(&generic_str_same).unwrap();
        let generic_str_diff_json = JsonSerde.serialize_command(&generic_str_diff).unwrap();

        assert_eq!(
            generic_val_json,
            JsonSerde.serialize_command(&generic_val).unwrap()
        );
        assert_eq!(generic_val_json, generic_val_same_json);
        assert_ne!(generic_val_json, generic_val_diff_json);
        assert_ne!(generic_val_json, a_json);
        assert_eq!(
            generic_str_json,
            JsonSerde.serialize_command(&generic_str).unwrap()
        );
        assert_eq!(generic_str_json, generic_str_same_json);
        assert_ne!(generic_str_json, generic_str_diff_json);
        assert_ne!(generic_str_json, a_json);

        register_event!(TestEventPayload);
        register_event!(TestEventGeneric<u128>);
        register_event!(TestEventGeneric<String>);
        register_event!(TestEventEnum);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_cmd = JsonSerde.deserialize_command(&cmd_json).unwrap();
        let new_cmd_second = JsonSerde.deserialize_command(&cmd_json).unwrap();
        let new_cmd_same = JsonSerde.deserialize_command(&same_json).unwrap();
        let new_cmd_val = JsonSerde.deserialize_command(&val_json).unwrap();
        let new_cmd_str = JsonSerde.deserialize_command(&str_json).unwrap();
        let new_cmd_a = JsonSerde.deserialize_command(&a_json).unwrap();
        let new_cmd_b = JsonSerde.deserialize_command(&b_json).unwrap();

        assert_eq!(cmd, new_cmd);
        assert_eq!(cmd, new_cmd_second);
        assert_eq!(cmd_same, new_cmd_same);
        assert_eq!(cmd_diff_val, new_cmd_val);
        assert_eq!(cmd_diff_str, new_cmd_str);
        assert_eq!(TestEventA.to_cmd(), new_cmd_a);
        assert_eq!(TestEventB.to_cmd(), new_cmd_b);

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(new_cmd_a, TestEventA.to_cmd());
        assert_eq!(new_cmd_b, TestEventB.to_cmd());

        let enum_new_a = JsonSerde.deserialize_command(&enum_a_json).unwrap();
        let enum_new_b = JsonSerde.deserialize_command(&enum_b_json).unwrap();
        let enum_new_b_diff = JsonSerde.deserialize_command(&enum_b_diff_json).unwrap();
        let enum_new_c = JsonSerde.deserialize_command(&enum_c_json).unwrap();
        let enum_new_c_diff = JsonSerde.deserialize_command(&enum_c_diff_json).unwrap();

        assert_eq!(enum_new_a, TestEventEnum::A.to_cmd());
        assert_eq!(enum_new_b, TestEventEnum::B(TEST_VAL).to_cmd());
        assert_eq!(enum_new_b_diff, TestEventEnum::B(TEST_VAL + 1).to_cmd());
        assert_eq!(enum_new_c, TestEventEnum::C(TEST_MSG.to_string()).to_cmd());
        assert_eq!(
            enum_new_c_diff,
            TestEventEnum::C(TEST_MSG[1..].to_string()).to_cmd()
        );

        let new_generic_val = JsonSerde.deserialize_command(&generic_val_json).unwrap();
        let new_generic_val_second = JsonSerde.deserialize_command(&generic_val_json).unwrap();
        let new_generic_val_same = JsonSerde
            .deserialize_command(&generic_val_same_json)
            .unwrap();
        let new_generic_val_diff = JsonSerde
            .deserialize_command(&generic_val_diff_json)
            .unwrap();
        let new_generic_str = JsonSerde.deserialize_command(&generic_str_json).unwrap();
        let new_generic_str_second = JsonSerde.deserialize_command(&generic_str_json).unwrap();
        let new_generic_str_same = JsonSerde
            .deserialize_command(&generic_str_same_json)
            .unwrap();
        let new_generic_str_diff = JsonSerde
            .deserialize_command(&generic_str_diff_json)
            .unwrap();

        assert_eq!(generic_val, new_generic_val);
        assert_eq!(generic_val, new_generic_val_second);
        assert_eq!(generic_val_same, new_generic_val_same);
        assert_eq!(generic_val_diff, new_generic_val_diff);
        assert_eq!(generic_str, new_generic_str);
        assert_eq!(generic_str, new_generic_str_second);
        assert_eq!(generic_str_same, new_generic_str_same);
        assert_eq!(generic_str_diff, new_generic_str_diff);

        assert_eq!(new_generic_val, new_generic_val_second);
        assert_eq!(new_generic_val, new_generic_val_same);
        assert_ne!(new_generic_val, new_generic_val_diff);
        assert_eq!(new_generic_str, new_generic_str_second);
        assert_eq!(new_generic_str, new_generic_str_same);
        assert_ne!(new_generic_str, new_generic_str_diff);

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
        assert_eq!(cmd_event_a, TestEventA);
        assert_eq!(cmd_event_b, TestEventB);

        let cmd_enum_a = enum_new_a.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_b = enum_new_b.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_b_diff = enum_new_b_diff.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_c = enum_new_c.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_c_diff = enum_new_c_diff.downcast_event::<TestEventEnum>().unwrap();

        assert_eq!(cmd_enum_a, TestEventEnum::A);
        assert_eq!(cmd_enum_b, TestEventEnum::B(TEST_VAL));
        assert_eq!(cmd_enum_b_diff, TestEventEnum::B(TEST_VAL + 1));
        assert_eq!(cmd_enum_c, TestEventEnum::C(TEST_MSG.to_string()));
        assert_eq!(cmd_enum_c_diff, TestEventEnum::C(TEST_MSG[1..].to_string()));

        let cmd_generic_val = new_generic_val
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_second = new_generic_val_second
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_same = new_generic_val_same
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_diff = new_generic_val_diff
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_str = new_generic_str
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_second = new_generic_str_second
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_same = new_generic_str_same
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_diff = new_generic_str_diff
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();

        assert_eq!(cmd_generic_val, cmd_generic_val_second);
        assert_eq!(cmd_generic_val, cmd_generic_val_same);
        assert_ne!(cmd_generic_val, cmd_generic_val_diff);
        assert_eq!(cmd_generic_str, cmd_generic_str_second);
        assert_eq!(cmd_generic_str, cmd_generic_str_same);
        assert_ne!(cmd_generic_str, cmd_generic_str_diff);
    }

    /// Test serialization and deserialization of events using binary format
    #[cfg(all(feature = "event", feature = "serde", feature = "binary"))]
    #[test]
    fn event_binary() {
        use crate::{register_event, BinarySerde, SerdeFormat};

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

        let generic_val = TestEventGeneric(TEST_VAL);
        let generic_val_same = TestEventGeneric(TEST_VAL);
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1);
        let generic_str = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string());
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string());

        let event_binary = BinarySerde.serialize_event(&event).unwrap();
        let same_binary = BinarySerde.serialize_event(&event_same).unwrap();
        let val_binary = BinarySerde.serialize_event(&event_diff_val).unwrap();
        let str_binary = BinarySerde.serialize_event(&event_diff_str).unwrap();
        let a_binary = BinarySerde.serialize_event(&TestEventA {}).unwrap();
        let b_binary = BinarySerde.serialize_event(&TestEventB {}).unwrap();

        assert_eq!(event_binary, BinarySerde.serialize_event(&event).unwrap());
        assert_eq!(event_binary, same_binary);
        assert_ne!(event_binary, val_binary);
        assert_ne!(event_binary, str_binary);
        assert_ne!(event_binary, a_binary);
        assert_ne!(a_binary, b_binary);

        let enum_a_binary = BinarySerde.serialize_event(&TestEventEnum::A).unwrap();
        let enum_b_binary = BinarySerde
            .serialize_event(&TestEventEnum::B(TEST_VAL))
            .unwrap();
        let enum_b_diff_binary = BinarySerde
            .serialize_event(&TestEventEnum::B(TEST_VAL + 1))
            .unwrap();
        let enum_c_binary = BinarySerde
            .serialize_event(&TestEventEnum::C(TEST_MSG.to_string()))
            .unwrap();
        let enum_c_diff_binary = BinarySerde
            .serialize_event(&TestEventEnum::C(TEST_MSG[1..].to_string()))
            .unwrap();

        assert_ne!(enum_a_binary, a_binary);
        assert_ne!(enum_a_binary, enum_b_binary);
        assert_ne!(enum_b_binary, enum_b_diff_binary);
        assert_ne!(enum_a_binary, enum_c_binary);
        assert_ne!(enum_c_binary, enum_c_diff_binary);
        assert_eq!(
            enum_a_binary,
            BinarySerde.serialize_event(&TestEventEnum::A).unwrap()
        );

        let generic_val_binary = BinarySerde.serialize_event(&generic_val).unwrap();
        let generic_val_same_binary = BinarySerde.serialize_event(&generic_val_same).unwrap();
        let generic_val_diff_binary = BinarySerde.serialize_event(&generic_val_diff).unwrap();
        let generic_str_binary = BinarySerde.serialize_event(&generic_str).unwrap();
        let generic_str_same_binary = BinarySerde.serialize_event(&generic_str_same).unwrap();
        let generic_str_diff_binary = BinarySerde.serialize_event(&generic_str_diff).unwrap();

        assert_eq!(
            generic_val_binary,
            BinarySerde.serialize_event(&generic_val).unwrap()
        );
        assert_eq!(generic_val_binary, generic_val_same_binary);
        assert_ne!(generic_val_binary, generic_val_diff_binary);
        assert_ne!(generic_val_binary, a_binary);
        assert_eq!(
            generic_str_binary,
            BinarySerde.serialize_event(&generic_str).unwrap()
        );
        assert_eq!(generic_str_binary, generic_str_same_binary);
        assert_ne!(generic_str_binary, generic_str_diff_binary);
        assert_ne!(generic_str_binary, a_binary);

        register_event!(TestEventPayload);
        register_event!(TestEventGeneric<u128>);
        register_event!(TestEventGeneric<String>);
        register_event!(TestEventEnum);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_event: TestEventPayload = BinarySerde.deserialize_event(&event_binary).unwrap();
        let new_same: TestEventPayload = BinarySerde.deserialize_event(&same_binary).unwrap();
        let new_val: TestEventPayload = BinarySerde.deserialize_event(&val_binary).unwrap();
        let new_str: TestEventPayload = BinarySerde.deserialize_event(&str_binary).unwrap();
        let new_a: TestEventA = BinarySerde.deserialize_event(&a_binary).unwrap();
        let new_b: TestEventB = BinarySerde.deserialize_event(&b_binary).unwrap();

        assert_eq!(event, new_event);
        assert_eq!(event_same, new_same);
        assert_eq!(event_diff_val, new_val);
        assert_eq!(event_diff_str, new_str);
        assert_eq!(TestEventA, new_a);
        assert_eq!(TestEventB, new_b);

        let enum_new_a: TestEventEnum = BinarySerde.deserialize_event(&enum_a_binary).unwrap();
        let enum_new_b: TestEventEnum = BinarySerde.deserialize_event(&enum_b_binary).unwrap();
        let enum_new_b_diff: TestEventEnum =
            BinarySerde.deserialize_event(&enum_b_diff_binary).unwrap();
        let enum_new_c: TestEventEnum = BinarySerde.deserialize_event(&enum_c_binary).unwrap();
        let enum_new_c_diff: TestEventEnum =
            BinarySerde.deserialize_event(&enum_c_diff_binary).unwrap();

        assert_eq!(enum_new_a, TestEventEnum::A);
        assert_eq!(enum_new_b, TestEventEnum::B(TEST_VAL));
        assert_eq!(enum_new_b_diff, TestEventEnum::B(TEST_VAL + 1));
        assert_eq!(enum_new_c, TestEventEnum::C(TEST_MSG.to_string()));
        assert_eq!(enum_new_c_diff, TestEventEnum::C(TEST_MSG[1..].to_string()));

        let new_generic_val: TestEventGeneric<u128> =
            BinarySerde.deserialize_event(&generic_val_binary).unwrap();
        let new_generic_val_same: TestEventGeneric<u128> = BinarySerde
            .deserialize_event(&generic_val_same_binary)
            .unwrap();
        let new_generic_val_diff: TestEventGeneric<u128> = BinarySerde
            .deserialize_event(&generic_val_diff_binary)
            .unwrap();
        let new_generic_str: TestEventGeneric<String> =
            BinarySerde.deserialize_event(&generic_str_binary).unwrap();
        let new_generic_str_same: TestEventGeneric<String> = BinarySerde
            .deserialize_event(&generic_str_same_binary)
            .unwrap();
        let new_generic_str_diff: TestEventGeneric<String> = BinarySerde
            .deserialize_event(&generic_str_diff_binary)
            .unwrap();

        assert_eq!(generic_val, new_generic_val);
        assert_eq!(generic_val_same, new_generic_val_same);
        assert_eq!(generic_val_diff, new_generic_val_diff);
        assert_eq!(generic_str, new_generic_str);
        assert_eq!(generic_str_same, new_generic_str_same);
        assert_eq!(generic_str_diff, new_generic_str_diff);
    }

    /// Test serialization and deserialization of commands using binary format
    #[cfg(all(
        feature = "command",
        feature = "event",
        feature = "serde",
        feature = "binary"
    ))]
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

        let generic_val = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_same = TestEventGeneric(TEST_VAL).to_cmd();
        let generic_val_diff = TestEventGeneric(TEST_VAL + 1).to_cmd();
        let generic_str = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_same = TestEventGeneric(TEST_MSG.to_string()).to_cmd();
        let generic_str_diff = TestEventGeneric(TEST_MSG[1..].to_string()).to_cmd();

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

        let enum_a_binary = BinarySerde
            .serialize_command(&TestEventEnum::A.to_cmd())
            .unwrap();
        let enum_b_binary = BinarySerde
            .serialize_command(&TestEventEnum::B(TEST_VAL).to_cmd())
            .unwrap();
        let enum_b_diff_binary = BinarySerde
            .serialize_command(&TestEventEnum::B(TEST_VAL + 1).to_cmd())
            .unwrap();
        let enum_c_binary = BinarySerde
            .serialize_command(&TestEventEnum::C(TEST_MSG.to_string()).to_cmd())
            .unwrap();
        let enum_c_diff_binary = BinarySerde
            .serialize_command(&TestEventEnum::C(TEST_MSG[1..].to_string()).to_cmd())
            .unwrap();

        assert_ne!(enum_a_binary, a_binary);
        assert_ne!(enum_a_binary, enum_b_binary);
        assert_ne!(enum_b_binary, enum_b_diff_binary);
        assert_ne!(enum_a_binary, enum_c_binary);
        assert_ne!(enum_c_binary, enum_c_diff_binary);
        assert_eq!(
            enum_a_binary,
            BinarySerde
                .serialize_command(&TestEventEnum::A.to_cmd())
                .unwrap()
        );

        let generic_val_binary = BinarySerde.serialize_command(&generic_val).unwrap();
        let generic_val_same_binary = BinarySerde.serialize_command(&generic_val_same).unwrap();
        let generic_val_diff_binary = BinarySerde.serialize_command(&generic_val_diff).unwrap();
        let generic_str_binary = BinarySerde.serialize_command(&generic_str).unwrap();
        let generic_str_same_binary = BinarySerde.serialize_command(&generic_str_same).unwrap();
        let generic_str_diff_binary = BinarySerde.serialize_command(&generic_str_diff).unwrap();

        assert_eq!(
            generic_val_binary,
            BinarySerde.serialize_command(&generic_val).unwrap()
        );
        assert_eq!(generic_val_binary, generic_val_same_binary);
        assert_ne!(generic_val_binary, generic_val_diff_binary);
        assert_ne!(generic_val_binary, a_binary);
        assert_eq!(
            generic_str_binary,
            BinarySerde.serialize_command(&generic_str).unwrap()
        );
        assert_eq!(generic_str_binary, generic_str_same_binary);
        assert_ne!(generic_str_binary, generic_str_diff_binary);
        assert_ne!(generic_str_binary, a_binary);

        register_event!(TestEventPayload);
        register_event!(TestEventGeneric<u128>);
        register_event!(TestEventGeneric<String>);
        register_event!(TestEventEnum);
        register_event!(TestEventA);
        register_event!(TestEventB);

        let new_cmd: Command = BinarySerde.deserialize_command(&cmd_binary).unwrap();
        let new_cmd_second: Command = BinarySerde.deserialize_command(&cmd_binary).unwrap();
        let new_cmd_same: Command = BinarySerde.deserialize_command(&same_binary).unwrap();
        let new_cmd_val: Command = BinarySerde.deserialize_command(&val_binary).unwrap();
        let new_cmd_str: Command = BinarySerde.deserialize_command(&str_binary).unwrap();
        let new_cmd_a: Command = BinarySerde.deserialize_command(&a_binary).unwrap();
        let new_cmd_b: Command = BinarySerde.deserialize_command(&b_binary).unwrap();

        assert_eq!(cmd, new_cmd);
        assert_eq!(cmd, new_cmd_second);
        assert_eq!(cmd_same, new_cmd_same);
        assert_eq!(cmd_diff_val, new_cmd_val);
        assert_eq!(cmd_diff_str, new_cmd_str);
        assert_eq!(TestEventA.to_cmd(), new_cmd_a);
        assert_eq!(TestEventB.to_cmd(), new_cmd_b);

        assert_eq!(new_cmd, new_cmd_second);
        assert_eq!(new_cmd, new_cmd_same);
        assert_ne!(new_cmd, new_cmd_val);
        assert_ne!(new_cmd, new_cmd_str);
        assert_eq!(new_cmd_a, TestEventA.to_cmd());
        assert_eq!(new_cmd_b, TestEventB.to_cmd());

        let enum_new_a = BinarySerde.deserialize_command(&enum_a_binary).unwrap();
        let enum_new_b = BinarySerde.deserialize_command(&enum_b_binary).unwrap();
        let enum_new_b_diff = BinarySerde
            .deserialize_command(&enum_b_diff_binary)
            .unwrap();
        let enum_new_c = BinarySerde.deserialize_command(&enum_c_binary).unwrap();
        let enum_new_c_diff = BinarySerde
            .deserialize_command(&enum_c_diff_binary)
            .unwrap();

        assert_eq!(enum_new_a, TestEventEnum::A.to_cmd());
        assert_eq!(enum_new_b, TestEventEnum::B(TEST_VAL).to_cmd());
        assert_eq!(enum_new_b_diff, TestEventEnum::B(TEST_VAL + 1).to_cmd());
        assert_eq!(enum_new_c, TestEventEnum::C(TEST_MSG.to_string()).to_cmd());
        assert_eq!(
            enum_new_c_diff,
            TestEventEnum::C(TEST_MSG[1..].to_string()).to_cmd()
        );

        let new_generic_val: Command = BinarySerde
            .deserialize_command(&generic_val_binary)
            .unwrap();
        let new_generic_val_second: Command = BinarySerde
            .deserialize_command(&generic_val_binary)
            .unwrap();
        let new_generic_val_same: Command = BinarySerde
            .deserialize_command(&generic_val_same_binary)
            .unwrap();
        let new_generic_val_diff: Command = BinarySerde
            .deserialize_command(&generic_val_diff_binary)
            .unwrap();
        let new_generic_str: Command = BinarySerde
            .deserialize_command(&generic_str_binary)
            .unwrap();
        let new_generic_str_second: Command = BinarySerde
            .deserialize_command(&generic_str_binary)
            .unwrap();
        let new_generic_str_same: Command = BinarySerde
            .deserialize_command(&generic_str_same_binary)
            .unwrap();
        let new_generic_str_diff: Command = BinarySerde
            .deserialize_command(&generic_str_diff_binary)
            .unwrap();

        assert_eq!(generic_val, new_generic_val);
        assert_eq!(generic_val, new_generic_val_second);
        assert_eq!(generic_val_same, new_generic_val_same);
        assert_eq!(generic_val_diff, new_generic_val_diff);
        assert_eq!(generic_str, new_generic_str);
        assert_eq!(generic_str, new_generic_str_second);
        assert_eq!(generic_str_same, new_generic_str_same);
        assert_eq!(generic_str_diff, new_generic_str_diff);

        assert_eq!(new_generic_val, new_generic_val_second);
        assert_eq!(new_generic_val, new_generic_val_same);
        assert_ne!(new_generic_val, new_generic_val_diff);
        assert_eq!(new_generic_str, new_generic_str_second);
        assert_eq!(new_generic_str, new_generic_str_same);
        assert_ne!(new_generic_str, new_generic_str_diff);

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
        assert_eq!(cmd_event_a, TestEventA);
        assert_eq!(cmd_event_b, TestEventB);

        let cmd_enum_a = enum_new_a.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_b = enum_new_b.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_b_diff = enum_new_b_diff.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_c = enum_new_c.downcast_event::<TestEventEnum>().unwrap();
        let cmd_enum_c_diff = enum_new_c_diff.downcast_event::<TestEventEnum>().unwrap();

        assert_eq!(cmd_enum_a, TestEventEnum::A);
        assert_eq!(cmd_enum_b, TestEventEnum::B(TEST_VAL));
        assert_eq!(cmd_enum_b_diff, TestEventEnum::B(TEST_VAL + 1));
        assert_eq!(cmd_enum_c, TestEventEnum::C(TEST_MSG.to_string()));
        assert_eq!(cmd_enum_c_diff, TestEventEnum::C(TEST_MSG[1..].to_string()));

        let cmd_generic_val = new_generic_val
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_second = new_generic_val_second
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_same = new_generic_val_same
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_val_diff = new_generic_val_diff
            .downcast_event::<TestEventGeneric<u128>>()
            .unwrap();
        let cmd_generic_str = new_generic_str
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_second = new_generic_str_second
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_same = new_generic_str_same
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();
        let cmd_generic_str_diff = new_generic_str_diff
            .downcast_event::<TestEventGeneric<String>>()
            .unwrap();

        assert_eq!(cmd_generic_val, cmd_generic_val_second);
        assert_eq!(cmd_generic_val, cmd_generic_val_same);
        assert_ne!(cmd_generic_val, cmd_generic_val_diff);
        assert_eq!(cmd_generic_str, cmd_generic_str_second);
        assert_eq!(cmd_generic_str, cmd_generic_str_same);
        assert_ne!(cmd_generic_str, cmd_generic_str_diff);
    }
}
