/// A trait for serializing and deserializing events and commands using specified formats.
pub trait SerdeFormat:
    Send + Sync + Clone + Default + PartialEq + std::any::Any + std::fmt::Debug + std::hash::Hash
{
    /// Serialize the passed event into a vector of bytes.
    fn serialize_event(
        &self,
        event: &dyn crate::Event,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    /// Deserialize an event as type T from the passed byte slice.
    fn deserialize_event<T>(&self, data: &[u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + for<'de> serde::Deserialize<'de>;

    fn deserialize_event_dyn(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn crate::Event>, Box<dyn std::error::Error>>;

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
    fn serialize_event(
        &self,
        event: &dyn crate::Event,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        serde_json::to_vec(&event).map_err(|e| e.into())
    }

    fn deserialize_event<T>(&self, data: &[u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + for<'de> serde::Deserialize<'de>,
    {
        serde_json::from_slice(&data).map_err(|e| e.into())
    }
    
    fn deserialize_event_dyn(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn crate::Event>, Box<dyn std::error::Error>> {
        todo!()
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
    fn serialize_event(&self, event: &dyn crate::Event) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    {
        bitcode::serialize(&event).map_err(|e| e.into())
    }

    fn deserialize_event<T>(&self, data: &[u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + for<'de> serde::Deserialize<'de>,
    {
        bitcode::deserialize(data).map_err(|e| e.into())
    }
    
    fn deserialize_event_dyn(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn crate::Event>, Box<dyn std::error::Error>> {
        todo!()
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
