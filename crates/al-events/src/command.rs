use crate::{define_message_kind, DynMessage};

define_message_kind! { Command }

/// The `Command` trait defines the required methods for command types to exist in the system
/// along with trait bounds that dont interfere with trait object usage.
pub trait Command: CommandHelpers + erased_serde::Serialize {
    //fn execute(self: Box<Self>); //TODO: add context parameter?
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_derive::MessageMarker;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, MessageMarker)]
    struct TestCommand {
        pub id: u32,
        pub name: String,
    }
    impl CommandMarker for TestCommand {}

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // Register the type with the registry
        try_register_command::<TestCommand>()?;

        // Create an instance and box it as a trait object
        let original = TestCommand {
            id: 42,
            name: "test".to_string(),
        };
        let boxed: Box<dyn Command> = Box::new(original.clone());

        // Serialize to JSON
        let json = serde_json::to_string(&boxed)?;
        eprintln!("Serialized JSON: {}", json);

        // Deserialize back to Box<dyn Command>
        let deserialized: Box<dyn Command> = serde_json::from_str(&json)?;

        // Downcast to concrete type and compare
        let downcast = deserialized
            .downcast_ref::<TestCommand>()
            .expect("Should be TestCommand");
        assert_eq!(downcast, &original);

        Ok(())
    }
}
