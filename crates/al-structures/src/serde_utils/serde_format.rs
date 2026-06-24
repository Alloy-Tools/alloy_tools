//! # Serialization Format Traits and Unified Format Enum
//!
//! This module defines the core traits and the `Format<T>` enum that allow
//! both serde‑based and non-serde, direct formats to be used
//! interchangeably for a given target type `T`.
//!
//! ## Format Traits
//!
//! | Trait | Dyn Compatible | Purpose |
//! |-------|-------------|---------|
//! | `SerializeFormat` | yes | Serialize a type‑erased `&dyn erased_serde::Serialize` into bytes. |
//! | `SerdeFormat` | yes | A `SerializeFormat` that can also produce an erased serde `Deserializer`. Works with **any** `T: DeserializeOwned`. |
//! | `ErasedDeserialize<T>` | yes | A dyn compatible trait for formats that **cannot** provide a serde `Deserializer`. They must register each concrete type they support via `register_type::<U>()`. |
//! | `DeserializeInto` | no | A concrete format’s ability to deserialize directly into a specific `T` (e.g. `BinaryFormat`). Used internally for per‑type registration. |
//!
//! ## `Format<T>` – The Unified Format
//!
//! The `Format<T>` enum has two variants:
//!
//! - `Format::Serde(Box<dyn SerdeFormat>)` – for any serde‑compatible format (JSON, MessagePack, …).
//! - `Format::Erased(Box<dyn ErasedDeserialize<T>>)` – for direct formats (like `bitcode`) that require explicit type registration.
//!
//! The system type `T` (e.g. `Box<dyn Any>`, `Box<dyn Message>`, `MyMessage`, …) never needs to implement `Deserialize`.
//! Instead, the *concrete types* you want to deserialize (like `Test`) are registered once
//! with a `TypeRegistry`, and for the `Erased` path also registered with the format itself.
//!
//! Both paths are used through a single `deserialize` call on the appropriate registry,
//! so the rest of the application is agnostic to which kind of format is in use.

use crate::traits::{AsAny, DynTypeName, TypeName};
#[cfg(any(feature = "collections", doc))]
use crate::TypeId;
use std::error::Error;
#[cfg(any(feature = "collections", doc))]
use std::{ops::Deref, sync::Arc};

#[cfg(any(feature = "collections", doc))]
pub use format::Format;
#[cfg(any(feature = "collections", doc))]
mod format {
    pub enum Format<T> {
        Serde(Box<dyn super::SerdeFormat>),
        Erased(Box<dyn super::ErasedDeserialize<T>>),
    }

    impl<T: 'static> Clone for Format<T> {
        fn clone(&self) -> Self {
            match self {
                Self::Serde(fmt) => Self::Serde(fmt.clone()),
                Self::Erased(fmt) => Self::Erased(fmt.clone()),
            }
        }
    }

    impl<T> super::SerializeFormat for Format<T> {
        fn serialize(
            &self,
            value: &dyn erased_serde::Serialize,
            writer: &mut dyn std::io::Write,
        ) -> Result<(), Box<dyn super::Error>> {
            match self {
                Format::Serde(f) => f.serialize(value, writer),
                Format::Erased(f) => f.serialize(value, writer),
            }
        }
    }

    impl<T> std::fmt::Debug for Format<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Serde(arg0) => f.debug_tuple("Serde").field(arg0).finish(),
                Self::Erased(arg0) => f.debug_tuple("As").field(arg0).finish(),
            }
        }
    }

    impl<T> super::DynTypeName for Format<T> {
        fn module_path(&self) -> &'static str {
            match self {
                Format::Serde(f) => f.module_path(),
                Format::Erased(f) => f.module_path(),
            }
        }

        fn type_with_generics(&self) -> String {
            match self {
                Format::Serde(f) => f.type_with_generics(),
                Format::Erased(f) => f.type_with_generics(),
            }
        }
    }
}

/// A serialization format that knows how to turn any erased value into bytes and back, without knowing the concrete type.
/// Implementations only need to know how to turn `&dyn erased_serde::Serialize` into bytes and back.
/// Object-safe to be stored like `Box<dyn SerdeFormat>` or `Arc<dyn SerdeFormat>`.
pub trait SerdeFormat:
    SerializeFormat + DeserializeSliceFormat + DeserializeReaderFormat + AsAny + DynTypeName
{
    fn clone_format(&self) -> Box<dyn SerdeFormat>;
}

impl<
        T: SerializeFormat
            + DeserializeSliceFormat
            + DeserializeReaderFormat
            + Clone
            + AsAny
            + DynTypeName,
    > SerdeFormat for T
{
    fn clone_format(&self) -> Box<dyn SerdeFormat> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn SerdeFormat> {
    fn clone(&self) -> Self {
        self.clone_format()
    }
}

impl<F: SerdeFormat> DeserializeInto for F {
    fn deserialize_slice_into<T: for<'de> serde::Deserialize<'de>>(
        &self,
        data: &[u8],
    ) -> Result<T, Box<dyn Error>> {
        T::deserialize(&mut *self.deserialize_slice(data)?).map_err(Into::into)
    }

    fn deserialize_reader_into<T: for<'de> serde::Deserialize<'de>>(
        &self,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn Error>> {
        T::deserialize(&mut *self.deserialize_reader(reader)?).map_err(Into::into)
    }
}

/// Type erased serialization
pub trait SerializeFormat: std::fmt::Debug + Send + Sync {
    fn is_human_readable(&self) -> bool {
        false
    }

    /// Serialize the given erased value into the writer.
    fn serialize(
        &self,
        value: &dyn erased_serde::Serialize,
        writer: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn Error>>;
}

pub trait DeserializeSliceFormat: SerializeFormat {
    /// Deserialize bytes into an erased deserializer.
    /// The caller can then use the deserializer with a concrete type or visitor.
    fn deserialize_slice<'de>(
        &self,
        data: &'de [u8],
    ) -> Result<Box<dyn erased_serde::Deserializer<'de> + 'de>, Box<dyn Error>>;
}

pub trait DeserializeReaderFormat: SerializeFormat {
    /// Deserialize a stream of bytes into an erased deserializer.
    /// The caller can then use the deserializer with a concrete type or visitor.
    fn deserialize_reader<'de>(
        &self,
        reader: &'de mut dyn std::io::Read,
    ) -> Result<Box<dyn erased_serde::Deserializer<'de> + 'de>, Box<dyn std::error::Error>>;
}

pub trait DeserializeInto: SerializeFormat {
    fn deserialize_slice_into<T: for<'de> serde::Deserialize<'de>>(
        &self,
        data: &[u8],
    ) -> Result<T, Box<dyn Error>>;

    fn deserialize_reader_into<T: for<'de> serde::Deserialize<'de>>(
        &self,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn Error>>;
}

#[cfg(any(feature = "collections", doc))]
pub trait ErasedDeserialize<T>:
    SerializeFormat + AsAny + DynTypeName + Send + Sync + 'static
{
    fn clone_format(&self) -> Box<dyn ErasedDeserialize<T>>;

    fn register<
        U: TypeName + for<'de> serde::Deserialize<'de>,
        R: Deref<Target = crate::TypeIdRegistry<K, I>>,
        K: crate::collections::storage::utils::keyed::KeyedHandle<Arc<str>, TypeId>,
        I: crate::collections::storage::utils::indexed::IndexedHandle<Arc<str>>,
    >(
        &self,
        type_registry: R,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, Box<dyn std::error::Error>>
    where
        Self: Sized,
        I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
        <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
        <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.register_named(U::type_with_generics(), type_registry, into_target)
    }

    fn register_named<
        U: for<'de> serde::Deserialize<'de>,
        R: Deref<Target = crate::TypeIdRegistry<K, I>>,
        K: crate::collections::storage::utils::keyed::KeyedHandle<Arc<str>, TypeId>,
        I: crate::collections::storage::utils::indexed::IndexedHandle<Arc<str>>,
    >(
        &self,
        name: impl AsRef<str>,
        type_registry: R,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<crate::TypeId, Box<dyn std::error::Error>>
    where
        Self: Sized,
        I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
        <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
        <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static;

    fn get_deserializer(
        &self,
        type_id: TypeId,
    ) -> Result<Option<crate::DirectFactory<T>>, Box<dyn std::error::Error>>;
}

impl<T: 'static> Clone for Box<dyn ErasedDeserialize<T>> {
    fn clone(&self) -> Self {
        self.clone_format()
    }
}
