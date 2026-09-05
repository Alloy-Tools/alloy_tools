use crate::{define_message_kind, DynMessage};

define_message_kind! { Command }

/// The `Command` trait defines the required methods for command types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Command: CommandHelpers + erased_serde::Serialize {
    //fn execute(self: Box<Self>); //TODO: add context parameter?
}

#[cfg(test)]
mod tests {
    use crate::{MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};

    use super::*;
    use al_derive::{MessageMarker, TypeName};
    use al_structures::serde_utils::formats::JsonFormat;
    use serde::{Deserialize, Serialize};

    //TODO: add variant derives so `#[command]` will derive `CommandMarker` and `MessageMarker` like the old `#[event]`
    #[derive(
        Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TypeName, MessageMarker,
    )]
    struct TestCommand {
        pub id: u32,
        pub name: String,
    }
    impl CommandMarker for TestCommand {}

    #[test]
    fn roundtrip() {
        // Register the format
        let f_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();

        // Register the type
        let t_id = try_register_command::<TestCommand>().unwrap();

        // Create an instance and box it as a trait object
        let original = TestCommand {
            id: 42,
            name: "test".to_string(),
        };
        let boxed = original.clone().to_msg();

        // Serialize to JSON
        let mut json = Vec::new();
        MESSAGE_FORMATS()
            .serialize(f_id, t_id, &boxed, &mut json)
            .unwrap();
        println!(
            "(format_id, type_id, Serialized JSON): ({}, {}, {})",
            u8::from_be_bytes([json[0]]),
            u32::from_be_bytes(json[1..5].try_into().unwrap()),
            str::from_utf8(&json[5..]).unwrap()
        );

        // Deserialize back to DynMessage::Command
        let deserialized = MESSAGE_FORMATS()
            .deserialize_slice(MESSAGE_TYPE_REGISTRY(), &json)
            .unwrap();

        println!("DynMessage: {:?}", deserialized);
        // Downcast to concrete type and compare
        let downcast = deserialized
            .as_command().unwrap()
            .downcast_ref::<TestCommand>()
            .expect("Should be TestCommand");
        assert_eq!(downcast, &original);
    }
}
