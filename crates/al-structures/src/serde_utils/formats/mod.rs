#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "json")]
pub use json::{JsonFormat, JsonReaderDeserializer, JsonSliceDeserializer};

#[cfg(feature = "binary")]
pub mod binary;

#[cfg(feature = "binary")]
pub use binary::BinaryFormat;
