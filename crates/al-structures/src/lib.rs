//! Helper structures used by Alloy crates for cancellation, enum utilities, noop wakers, ect.

#![cfg_attr(docsrs, feature(doc_cfg))]

// map `self` to `al_structures` allowing the use of derive macros that use `al_structures::..`
extern crate self as al_structures;

#[cfg(feature = "paste")]
pub use paste::paste;

#[cfg(any(feature = "cancellation", doc))]
pub mod cancellation;

#[cfg(any(feature = "enums", doc))]
pub mod enums;

#[cfg(any(feature = "traits", doc))]
pub mod traits;

#[cfg(any(feature = "noop_waker", doc))]
pub mod noop_waker;

#[cfg(any(feature = "race", doc))]
mod race;
#[cfg(any(feature = "race", doc))]
pub use race::Race;

#[cfg(any(feature = "collections", doc))]
pub mod collections;

#[cfg(any(feature = "serde_utils", doc))]
mod serde_utils;
#[cfg(any(all(feature = "serde_utils", feature = "collections"), doc))]
pub use serde_utils::visitor::GenericRegistryVisitor;

#[cfg(any(feature = "serde_format", doc))]
pub use serde_format_gate::*;
#[cfg(any(feature = "serde_format", doc))]
mod serde_format_gate {
    pub use super::serde_utils::serde_format::{
        DeserializeInto, DeserializeReaderFormat, DeserializeSliceFormat, SerdeFormat,
        SerializeFormat,
    };

    #[cfg(any(feature = "collections", doc))]
    pub use super::serde_utils::{
        deserializer_fns::{DeserializeFromBytesFn, DeserializeFromDeFn},
        serde_format::{ErasedDeserialize, Format},
        serde_registries::{
            DirectFactory, ErasedFactory, ErasedTypeRegistry, FormatId, FormatRegistry,
            FormatTypeRegistry, SerdeFactory, SerdeTypeRegistry, TypeId, TypeIdRegistry,
        },
    };

    #[cfg(any(feature = "json", doc))]
    pub use super::serde_utils::formats::json::{
        JsonFormat, JsonReaderDeserializer, JsonSliceDeserializer,
    };

    #[cfg(any(feature = "binary", doc))]
    pub use super::serde_utils::formats::binary::BinaryFormat;
}
