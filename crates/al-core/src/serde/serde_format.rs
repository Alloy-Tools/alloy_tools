/// A trait for serializing and deserializing events and commands using specified formats.
pub trait SerdeFormat:
    Send + Sync + Clone + Default + PartialEq + std::any::Any + std::fmt::Debug + std::hash::Hash
{
    /// Serialize the passed event into a vector of bytes.
    fn serialize_event<T>(&self, event: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Serialize;

    /// Deserialize an event as type T from the passed byte slice.
    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>;

    /// Serialize the passed command into a vector of bytes.
    fn serialize_command(
        &self,
        command: &crate::Command,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    /// Deserialize a command from the passed byte slice.
    fn deserialize_command<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<crate::Command, Box<dyn std::error::Error>>;
}

/// A JSON-based implementation of the SerdeFormat trait using serde_json.
#[cfg(feature = "json")]
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub struct JsonSerde;

#[cfg(feature = "json")]
impl SerdeFormat for JsonSerde {
    fn serialize_event<T>(&self, event: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Serialize,
    {
        serde_json::to_vec(event).map_err(|e| e.into())
    }

    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>,
    {
        serde_json::from_slice(&data).map_err(|e| e.into())
    }

    fn serialize_command(
        &self,
        command: &crate::Command,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        serde_json::to_vec(command).map_err(|e| e.into())
    }

    fn deserialize_command<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<crate::Command, Box<dyn std::error::Error>> {
        serde_json::from_slice(data).map_err(|e| e.into())
    }
}

/// A binary-based implementation of the SerdeFormat trait using bitcode.
#[cfg(feature = "binary")]
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub struct BinarySerde;

#[cfg(feature = "binary")]
impl SerdeFormat for BinarySerde {
    fn serialize_event<T>(&self, event: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Serialize,
    {
        bitcode::serialize(event).map_err(|e| e.into())
    }

    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>,
    {
        bitcode::deserialize(data).map_err(|e| e.into())
    }

    fn serialize_command(
        &self,
        command: &crate::Command,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        bitcode::serialize(command).map_err(|e| e.into())
    }

    fn deserialize_command<'a>(
        &self,
        data: &'a [u8],
    ) -> Result<crate::Command, Box<dyn std::error::Error>> {
        bitcode::deserialize(data).map_err(|e| e.into())
    }
}
