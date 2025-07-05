pub trait EventDeserializer: Send + Sync {
    /// Deserialize an event from the given deserializer.
    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>;
}

#[cfg(feature = "json")]
struct JsonDeserializer;

#[cfg(feature = "json")]
impl EventDeserializer for JsonDeserializer {
    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>,
    {
        serde_json::from_slice(data).map_err(|e| e.into())
    }
}

#[cfg(feature = "binary")]
struct BinaryDeserializer;

#[cfg(feature = "binary")]
impl EventDeserializer for BinaryDeserializer {
    fn deserialize_event<'a, T>(&self, data: &'a [u8]) -> Result<T, Box<dyn std::error::Error>>
    where
        T: crate::Event + serde::Deserialize<'a>,
    {
        bitcode::deserialize(data).map_err(|e| e.into())
    }
}