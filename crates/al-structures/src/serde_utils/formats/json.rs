use crate::serde_utils::serde_format::{
    DeserializeReaderFormat, DeserializeSliceFormat, Format, SerializeFormat,
};
use al_derive::TypeName;
use std::error::Error;
use std::io::Write;

crate::impl_erased_deserializer!(
    JsonSliceDeserializer,
    serde_json::Deserializer<serde_json::de::SliceRead<'de>>
);

crate::impl_erased_deserializer!(
    JsonReaderDeserializer,
    serde_json::Deserializer<serde_json::de::IoRead<&'de mut dyn std::io::Read>>
);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, TypeName)]
pub struct JsonFormat;

impl<T> From<JsonFormat> for Format<T> {
    fn from(value: JsonFormat) -> Self {
        Self::Serde(Box::new(value))
    }
}

impl SerializeFormat for JsonFormat {
    fn is_human_readable(&self) -> bool {
        true
    }

    fn serialize(
        &self,
        value: &dyn erased_serde::Serialize,
        writer: &mut dyn Write,
    ) -> Result<(), Box<dyn Error>> {
        erased_serde::serialize(value, &mut serde_json::Serializer::new(writer))?;
        Ok(())
    }
}

impl DeserializeSliceFormat for JsonFormat {
    fn deserialize_slice<'de>(
        &self,
        data: &'de [u8],
    ) -> Result<Box<dyn erased_serde::Deserializer<'de> + 'de>, Box<dyn Error>> {
        Ok(Box::new(<dyn erased_serde::Deserializer>::erase(
            JsonSliceDeserializer::new(serde_json::Deserializer::from_slice(data)),
        )))
    }
}

impl DeserializeReaderFormat for JsonFormat {
    fn deserialize_reader<'de>(
        &self,
        reader: &'de mut dyn std::io::Read,
    ) -> Result<Box<dyn erased_serde::Deserializer<'de> + 'de>, Box<dyn std::error::Error>> {
        Ok(Box::new(<dyn erased_serde::Deserializer>::erase(
            JsonReaderDeserializer::new(serde_json::Deserializer::from_reader(reader)),
        )))
    }
}
