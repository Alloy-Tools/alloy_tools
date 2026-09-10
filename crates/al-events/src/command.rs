use crate::{define_message_kind, DynMessage};

define_message_kind! { Command }

/// The `Command` trait defines the required methods for command types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Command: CommandHelpers + erased_serde::Serialize {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};
    use al_derive::command;
    use al_structures::serde_utils::formats::JsonFormat;

    #[command]
    struct TestCommand {
        pub id: u32,
        pub name: String,
    }

    #[test]
    fn slice() {
        // Register the format
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();

        // Register the type
        let type_id = try_register_command::<TestCommand>().unwrap();

        // Create an instance and box it as a trait object
        let original = TestCommand {
            id: 42,
            name: "slice".to_string(),
        };
        let boxed = original.clone().to_msg();
        let meta = boxed.as_command().and_then(|(_, m)| Some(m)).unwrap();

        // Serialize to `(format_id, type_id, JSON)`
        let mut encoded = Vec::new();
        MESSAGE_FORMATS()
            .serialize_registered(MESSAGE_TYPE_REGISTRY(), format_id, type_id, &boxed, &mut encoded)
            .unwrap();

        // Deserialize back to DynMessage::Command
        let decoded = MESSAGE_FORMATS()
            .deserialize_slice(MESSAGE_TYPE_REGISTRY(), &encoded)
            .unwrap();
        assert_eq!(
            meta,
            decoded.as_command().and_then(|(_, m)| Some(m)).unwrap()
        );

        // Downcast to concrete type and compare
        let downcast = decoded
            .as_command()
            .and_then(|(c, _)| c.downcast_ref::<TestCommand>())
            .unwrap();
        assert_eq!(downcast, &original);
    }

    #[test]
    fn reader() {
        let format_id = MESSAGE_FORMATS().register(JsonFormat).unwrap();
        let type_id = try_register_command::<TestCommand>().unwrap();
        let original = TestCommand {
            id: 42,
            name: "reader".to_string(),
        };

        let boxed = original.clone().to_msg();
        let meta = boxed.as_command().and_then(|(_, m)| Some(m)).unwrap();

        let mut encoded = Vec::new();
        MESSAGE_FORMATS()
            .serialize_registered(MESSAGE_TYPE_REGISTRY(), format_id, type_id, &boxed, &mut encoded)
            .unwrap();

        let decoded = MESSAGE_FORMATS()
            .deserialize_reader(MESSAGE_TYPE_REGISTRY(), &mut encoded.as_slice())
            .unwrap();
        assert_eq!(
            meta,
            decoded.as_command().and_then(|(_, m)| Some(m)).unwrap()
        );

        let downcast = decoded
            .as_command()
            .and_then(|(c, _)| c.downcast_ref::<TestCommand>())
            .unwrap();
        assert_eq!(downcast, &original);
    }
}
