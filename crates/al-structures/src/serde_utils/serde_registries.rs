//! # Deserialization Registries
//!
//! This module provides the infrastructure to map **type names** to **factories**
//! that know how to deserialize a specific concrete type from a specific format.
//! Everything is designed for runtime extension – new formats and new types can be
//! added dynamically without modifying existing code.
//!
//! ## Key Components
//!
//! ### Identifiers
//! - `TypeId` – a `u32` uniquely identifying a concrete type within the registry, allowing 4.92+ billion types to be loaded at once.
//! - `FormatId` – a `u8` uniquely identifying a format instance, allowing 256 formats to be loaded at once.
//!
//! ### Type ID Registry
//! `TypeIdRegistry<M, R>` maps type names (strings) to `TypeId`s.
//!
//! ### Serde‑Path Registry
//! `SerdeTypeRegistry<T, R, K, I, M, U>` stores factories of the form
//! `Arc<dyn Fn(&dyn SerdeFormat, &[u8]) -> Result<T, Box<dyn Error>>>`.
//! A factory is registered once per concrete type and works with **any** `SerdeFormat`.
//! This path has no per‑format registration overhead – it uses the format’s own
//! `deserialize_slice` to obtain a serde `Deserializer`.
//!
//! ### Erased‑Path Helper
//! `ErasedTypeRegistry<T>` is a stateless helper that handles deserialization for the
//! `Format::Erased` variant. It calls `format.get_deserializer(type_id)` to obtain a
//! pre‑registered function that knows how to deserialize the type using that specific format.
//!
//! ### Unified Coordinator
//! `FormatTypeRegistry<T, R, K, I, M, S>` combines both `SerdeTypeRegistry` and `ErasedTypeRegistry`.
//! Its `deserialize` method dispatches on the `Format<T>` enum automatically,
//! giving a single entry point for any format.
//!
//! ### Format Registry
//! `FormatRegistry<T, I, M>` manages concrete `Format<T>` instances (serde or erased),
//! mapping format names to `FormatId`s and storing the `Format<T>` values.
//!
//! ## Typical Usage
//! 1. Create a `TypeIdRegistry` to pass to the format type registry.
//! 2. Create a `FormatTypeRegistry` (or use the individual `SerdeTypeRegistry`/`ErasedTypeRegistry` directly).
//! 3. Create a `FormatRegistry` and register your formats (e.g., JSON, Binary).
//!     - For erased formats, each concrete type you want to deserialize must be registered with the format itself.
//! 4. Register each concrete type you want to deserialize with the `FormatTypeRegistry`.
//! 5. At runtime, call `registry.deserialize(data)` which will pull out the `(FormatId, TypeId, &[u8])`, deserialize the type, and return it as a homogenous `T`.
use std::{marker::PhantomData, ops::Deref, sync::Arc};

use crate::{
    collections::storage::utils::{indexed::IndexedHandle, keyed::KeyedHandle, HandleError},
    serde_utils::serde_format::{ErasedDeserialize, Format, SerdeFormat, SerializeFormat},
    traits::{DynTypeName, TypeName},
};

pub trait TypeDispatcher<T> {
    fn deserialize_slice(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        data: &[u8],
    ) -> Result<T, HandleError>;
    fn deserialize_reader(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, HandleError>;
}

impl<
        T: 'static,
        R: Deref<Target = TypeIdRegistry<K, I>> + Clone,
        K: KeyedHandle<Arc<str>, TypeId>,
        I: IndexedHandle<Arc<str>>,
        M: KeyedHandle<TypeId, S::Key>,
        S: IndexedHandle<SerdeFactory<T>>,
    > TypeDispatcher<T> for FormatTypeRegistry<T, R, K, I, M, S>
where
    S::Key: Eq + Clone,
    I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
    <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
{
    fn deserialize_slice(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        data: &[u8],
    ) -> Result<T, HandleError> {
        FormatTypeRegistry::deserialize_slice(self, format, type_id, data)
            .map_err(|error| HandleError::Deserialization(error.to_string()))
    }

    fn deserialize_reader(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, HandleError> {
        FormatTypeRegistry::deserialize_reader(self, format, type_id, reader)
            .map_err(|error| HandleError::Deserialization(error.to_string()))
    }
}

pub trait BeBytes: Sized {
    const LEN: usize;
    fn to_be_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;
    fn from_be_bytes(buffer: &[u8]) -> Result<Self, HandleError>;
}

pub trait PayloadHeader {
    const BUF_LEN: usize;
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;

    fn decode(buffer: &[u8]) -> Result<Self, HandleError>
    where
        Self: Sized;

    fn decode_at(offset: &mut usize, buffer: &[u8]) -> Result<Self, HandleError>
    where
        Self: Sized,
    {
        let target = offset
            .checked_add(Self::BUF_LEN)
            .ok_or_else(|| HandleError::Deserialization("header offset overflow".into()))?;
        if target > buffer.len() {
            return Err(HandleError::Deserialization(format!(
                "Buffer length too small for header with end index '{target}.'"
            )));
        }
        let res = Self::decode(&buffer[*offset..target]);
        if res.is_ok() {
            *offset = target;
        }
        res
    }
}

impl<T: BeBytes> PayloadHeader for T {
    const BUF_LEN: usize = T::LEN;

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.to_be_bytes(writer)
    }

    fn decode(buffer: &[u8]) -> Result<Self, HandleError>
    where
        Self: Sized,
    {
        if buffer.len() != Self::BUF_LEN {
            return Err(HandleError::Deserialization(format!(
                "Expected '{}' bytes, got '{}'",
                Self::BUF_LEN,
                buffer.len()
            )));
        }
        Self::from_be_bytes(buffer)
    }
}

// ----- Format Registry -----
// Allows 256 possible formats to be loaded at once for one byte.
pub type FormatId = u8;

impl BeBytes for FormatId {
    const LEN: usize = 1;

    fn to_be_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&u8::to_be_bytes(*self))
    }

    fn from_be_bytes(buffer: &[u8]) -> Result<Self, HandleError> {
        Ok(Self::from_be_bytes(buffer.try_into().map_err(|e| {
            HandleError::Deserialization(format!("{e}"))
        })?))
    }
}

pub struct FormatRegistry<T, I: IndexedHandle<Format<T>>, M: KeyedHandle<String, I::Key>> {
    inner: I,
    id_map: M,
    _phantom: PhantomData<T>,
}

impl<T: 'static, I: IndexedHandle<Format<T>>, M: KeyedHandle<String, I::Key>>
    FormatRegistry<T, I, M>
where
    I::Key: Eq + Clone + TryInto<FormatId> + From<FormatId>,
    <I::Key as TryInto<FormatId>>::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new(inner: I, id_map: M) -> Self {
        Self {
            inner,
            id_map,
            _phantom: PhantomData,
        }
    }

    pub fn register(
        &self,
        format: impl Into<Format<T>> + 'static,
    ) -> Result<FormatId, HandleError> {
        let format = format.into();
        self.register_named(format.type_with_generics(), format)
    }

    pub fn register_named(
        &self,
        name: impl Into<String>,
        format: impl Into<Format<T>> + 'static,
    ) -> Result<FormatId, HandleError> {
        let name = name.into();
        if let Some(id) = self.id_map.get(&name)? {
            return Ok(id
                .try_into()
                .map_err(|e| HandleError::ConversionFailed(e.into()))?);
        }

        let id = self.inner.push(format.into())?;
        self.id_map.insert(name, id.clone())?;
        Ok(id
            .try_into()
            .map_err(|e| HandleError::ConversionFailed(e.into()))?)
    }

    pub fn get_format(&self, id: FormatId) -> Result<Option<Format<T>>, HandleError> {
        self.inner.get(&I::Key::from(id))
    }

    pub fn get_format_by_name(
        &self,
        name: impl Into<String>,
    ) -> Result<Option<Format<T>>, HandleError> {
        let name = name.into();
        if let Some(id) = self.id_map.get(&name)? {
            return self.inner.get(&id);
        }
        Ok(None)
    }

    pub fn serialize<S: erased_serde::Serialize>(
        &self,
        format_id: FormatId,
        type_id: TypeId,
        value: &S,
        mut writer: &mut dyn std::io::Write,
    ) -> Result<(), HandleError> {
        let format = self.inner.get(&I::Key::from(format_id))?.ok_or_else(|| {
            HandleError::Custom(format!("No format found for the id '{format_id}'").into())
        })?;
        // Write with format `(format_id, type_id, payload_bytes)`
        format_id.encode(&mut writer)?;
        type_id.encode(&mut writer)?;
        format
            .serialize(value, writer)
            .map_err(|e| HandleError::Serialization(e.to_string()))
    }

    pub fn deserialize_slice<D: Deref<Target = R>, R: TypeDispatcher<T>>(
        &self,
        format_type_registry: D,
        slice: &[u8],
    ) -> Result<T, HandleError> {
        let mut offset = 0;
        let format_id = FormatId::decode_at(&mut offset, &slice)?;
        let type_id = TypeId::decode_at(&mut offset, &slice)?;
        let format = self.get_format(format_id)?.ok_or_else(|| {
            HandleError::Deserialization(format!("No format found for id '{format_id}'"))
        })?;

        format_type_registry.deserialize_slice(&format, type_id, &slice[offset..])
    }

    pub fn deserialize_reader<D: Deref<Target = R>, R: TypeDispatcher<T>>(
        &self,
        format_type_registry: D,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, HandleError> {
        let mut header = [0u8; FormatId::BUF_LEN + TypeId::BUF_LEN];
        reader.read_exact(&mut header)?;

        let mut offset = 0;
        let format_id = FormatId::decode_at(&mut offset, &header)?;
        let type_id = TypeId::decode_at(&mut offset, &header)?;
        let format = self.get_format(format_id)?.ok_or_else(|| {
            HandleError::Deserialization(format!("No format found for id '{format_id}'"))
        })?;

        format_type_registry.deserialize_reader(&format, type_id, reader)
    }
}

pub struct FormatTypeRegistry<
    T: 'static,
    R: Deref<Target = TypeIdRegistry<K, I>> + Clone,
    K: KeyedHandle<Arc<str>, TypeId>,
    I: IndexedHandle<Arc<str>>,
    M: KeyedHandle<TypeId, S::Key>,
    S: IndexedHandle<SerdeFactory<T>>,
> {
    type_registry: R,
    serde: SerdeTypeRegistry<T, R, K, I, M, S>,
    erased: ErasedTypeRegistry<T>,
}

impl<
        T: 'static,
        R: Deref<Target = TypeIdRegistry<K, I>> + Clone,
        K: KeyedHandle<Arc<str>, TypeId>,
        I: IndexedHandle<Arc<str>>,
        M: KeyedHandle<TypeId, S::Key>,
        S: IndexedHandle<SerdeFactory<T>>,
    > FormatTypeRegistry<T, R, K, I, M, S>
where
    S::Key: Eq + Clone,
    I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
    <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new(type_registry: R, id_map: M, serde_inner: S) -> Self {
        Self {
            type_registry: type_registry.clone(),
            serde: SerdeTypeRegistry::new(type_registry, id_map, serde_inner),
            erased: ErasedTypeRegistry::new(),
        }
    }

    pub fn register<U: TypeName + for<'de> serde::Deserialize<'de>>(
        &self,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, HandleError>
    where
        U: 'static,
        T: 'static,
    {
        self.register_named(U::type_with_generics(), into_target)
    }

    pub fn register_named<U: for<'de> serde::Deserialize<'de>>(
        &self,
        name: impl AsRef<str>,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, HandleError>
    where
        U: 'static,
        T: 'static,
    {
        self.serde.register_named(name, into_target)
    }

    pub fn get_serde_factory(
        &self,
        type_id: TypeId,
    ) -> Result<Option<SerdeFactory<T>>, HandleError> {
        self.serde.get_factory(type_id)
    }

    pub fn get_serde_factory_by_name(
        &self,
        name: impl AsRef<str>,
    ) -> Result<Option<SerdeFactory<T>>, HandleError> {
        self.serde.get_factory_by_name(name)
    }

    pub fn get_erased_factory(
        &self,
        format: &dyn ErasedDeserialize<T>,
        type_id: TypeId,
    ) -> Result<Option<DirectFactory<T>>, Box<dyn std::error::Error>> {
        format.get_deserializer(type_id)
    }

    pub fn get_erased_factory_by_name(
        &self,
        format: &dyn ErasedDeserialize<T>,
        name: impl Into<String>,
    ) -> Result<Option<DirectFactory<T>>, Box<dyn std::error::Error>> {
        let name = name.into();
        let name_clone = name.clone();
        let type_id = self.type_registry.get_id_by_name(name)?.ok_or_else(|| {
            format!("Type '{name_clone}' not registered with the `TypeId` registry")
        })?;
        format.get_deserializer(type_id)
    }

    pub fn deserialize_slice(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        data: &[u8],
    ) -> Result<T, Box<dyn std::error::Error>> {
        match format {
            Format::Serde(s) => self.serde.deserialize_slice(s.as_ref(), type_id, data),
            Format::Erased(e) => self.erased.deserialize_slice(e.as_ref(), type_id, data),
        }
    }

    pub fn deserialize_reader(
        &self,
        format: &Format<T>,
        type_id: TypeId,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn std::error::Error>> {
        match format {
            Format::Serde(s) => self.serde.deserialize_reader(s.as_ref(), type_id, reader),
            Format::Erased(e) => self.erased.deserialize_reader(e.as_ref(), type_id, reader),
        }
    }
}

// ----- SerdeFormat Registry -----
pub type SerdeFactory<T> = TypeFactory<T, dyn SerdeFormat>;
pub struct SerdeTypeRegistry<
    T,
    R: Deref<Target = TypeIdRegistry<K, I>>,
    K: KeyedHandle<Arc<str>, TypeId>,
    I: IndexedHandle<Arc<str>>,
    M: KeyedHandle<TypeId, U::Key>,
    U: IndexedHandle<SerdeFactory<T>>,
> {
    type_registry: R,
    id_map: M,
    registry: U,
    _phantom: PhantomData<T>,
}

impl<
        T,
        R: Deref<Target = TypeIdRegistry<K, I>>,
        K: KeyedHandle<Arc<str>, TypeId>,
        I: IndexedHandle<Arc<str>>,
        M: KeyedHandle<TypeId, _U::Key>,
        _U: IndexedHandle<SerdeFactory<T>>,
    > SerdeTypeRegistry<T, R, K, I, M, _U>
where
    _U::Key: Eq + Clone,
    I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
    <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new(type_registry: R, id_map: M, registry: _U) -> Self {
        Self {
            type_registry,
            id_map,
            registry,
            _phantom: PhantomData,
        }
    }

    pub fn register<U: TypeName + for<'de> serde::Deserialize<'de>>(
        &self,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, HandleError>
    where
        U: 'static,
        T: 'static,
    {
        self.register_named::<U>(U::type_with_generics(), into_target)
    }

    pub fn register_named<U: for<'de> serde::Deserialize<'de>>(
        &self,
        name: impl AsRef<str>,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, HandleError>
    where
        U: 'static,
        T: 'static,
    {
        let into_target = Arc::new(into_target);
        let into_target_clone = into_target.clone();
        let type_id = self.type_registry.register_named(name)?;

        let slice_factory = Arc::new(move |fmt: &dyn SerdeFormat, data: &[u8]| {
            let mut de = fmt.deserialize_slice(data)?;
            Ok(into_target(U::deserialize(&mut *de)?))
        });
        let reader_factory = Arc::new(
            move |fmt: &dyn SerdeFormat, reader: &mut dyn std::io::Read| {
                let mut de = fmt.deserialize_reader(reader)?;
                Ok(into_target_clone(U::deserialize(&mut *de)?))
            },
        );

        let factory_id = self
            .registry
            .push(TypeFactory::new(slice_factory, reader_factory))?;
        self.id_map.insert(type_id, factory_id)?;
        Ok(type_id)
    }

    pub fn get_factory(&self, id: TypeId) -> Result<Option<SerdeFactory<T>>, HandleError> {
        match self.id_map.get(&id)? {
            Some(f_id) => self.registry.get(&f_id),
            None => Ok(None),
        }
    }

    pub fn get_factory_by_name(
        &self,
        name: impl AsRef<str>,
    ) -> Result<Option<SerdeFactory<T>>, HandleError> {
        match self.type_registry.get_id_by_name(name)? {
            Some(id) => self.get_factory(id),
            None => Ok(None),
        }
    }

    pub fn deserialize_slice(
        &self,
        format: &dyn SerdeFormat,
        type_id: TypeId,
        data: &[u8],
    ) -> Result<T, Box<dyn std::error::Error>> {
        let factory = self
            .get_factory(type_id)?
            .ok_or_else(|| format!("Factory not found for {type_id}"))?;
        (factory.slice)(format, data)
    }

    pub fn deserialize_reader(
        &self,
        format: &dyn SerdeFormat,
        type_id: TypeId,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let factory = self
            .get_factory(type_id)?
            .ok_or_else(|| format!("Factory not found for {type_id}"))?;
        (factory.reader)(format, reader)
    }
}

// ----- Erased Registry -----
pub type ErasedFactory<T> = TypeFactory<T, dyn ErasedDeserialize<T>>;
pub type DirectFactory<T> = TypeFactory<T, dyn std::any::Any>;

pub struct ErasedTypeRegistry<T: 'static> {
    _phantom: PhantomData<T>,
}

impl<T> ErasedTypeRegistry<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    pub fn deserialize_slice(
        &self,
        format: &dyn ErasedDeserialize<T>,
        type_id: TypeId,
        data: &[u8],
    ) -> Result<T, Box<dyn std::error::Error>> {
        let factory = format
            .get_deserializer(type_id)?
            .ok_or_else(|| format!("No factory registered for type with id '{type_id}'"))?;
        (factory.slice)(format.as_any(), data)
    }

    pub fn deserialize_reader(
        &self,
        format: &dyn ErasedDeserialize<T>,
        type_id: TypeId,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let factory = format
            .get_deserializer(type_id)?
            .ok_or_else(|| format!("No factory registered for type with id '{type_id}'"))?;
        (factory.reader)(format.as_any(), reader)
    }
}

#[macro_export]
macro_rules! register_type {
    ($self:expr, $type_registry:expr, $into_target:expr, $name:expr, $concrete_type:ty, $error_msg:expr) => {{
        let into_target = std::sync::Arc::new($into_target);
        let into_target_clone = into_target.clone();
        let type_id = $type_registry.register_named($name)?;

        let slice_deser_fn = move |fmt: &dyn std::any::Any, data: &[u8]| {
            let me = fmt
                .downcast_ref::<$concrete_type>()
                .ok_or_else(|| $error_msg)?;
            let value: U = me.deserialize_slice_into(data)?;
            Ok(into_target(value))
        };
        let reader_deser_fn = move |fmt: &dyn std::any::Any, reader: &mut dyn std::io::Read| {
            let me = fmt
                .downcast_ref::<$concrete_type>()
                .ok_or_else(|| $error_msg)?;
            let value: U = me.deserialize_reader_into(reader)?;
            Ok(into_target_clone(value))
        };

        $self.deserializers.try_insert(
            type_id,
            $crate::serde_utils::serde_registries::TypeFactory::new(
                std::sync::Arc::new(slice_deser_fn),
                std::sync::Arc::new(reader_deser_fn),
            ),
        )?;
        Ok(type_id)
    }};
}

// ----- Type Registry -----
// Allows 4.92+ billion types to be loaded at once at the cost of 4 bytes.
pub type TypeId = u32;

impl BeBytes for TypeId {
    const LEN: usize = 4;

    fn to_be_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&u32::to_be_bytes(*self))
    }

    fn from_be_bytes(buffer: &[u8]) -> Result<Self, HandleError> {
        Ok(u32::from_be_bytes(buffer.try_into().map_err(|e| {
            HandleError::Deserialization(format!("{e}"))
        })?))
    }
}

type TypeSliceFactory<T, F> =
    Arc<dyn Fn(&F, &[u8]) -> Result<T, Box<dyn std::error::Error>> + Send + Sync>;
type TypeReaderFactory<T, F> = Arc<
    dyn for<'a> Fn(&F, &'a mut dyn std::io::Read) -> Result<T, Box<dyn std::error::Error>>
        + Send
        + Sync,
>;

pub struct TypeFactory<T, F: ?Sized> {
    slice: TypeSliceFactory<T, F>,
    reader: TypeReaderFactory<T, F>,
}

impl<T, F: ?Sized> From<(TypeSliceFactory<T, F>, TypeReaderFactory<T, F>)> for TypeFactory<T, F> {
    fn from(value: (TypeSliceFactory<T, F>, TypeReaderFactory<T, F>)) -> Self {
        let (slice, reader) = value;
        TypeFactory::new(slice, reader)
    }
}

impl<T, F: ?Sized> Clone for TypeFactory<T, F> {
    fn clone(&self) -> Self {
        Self {
            slice: self.slice.clone(),
            reader: self.reader.clone(),
        }
    }
}

impl<T, F: ?Sized> TypeFactory<T, F> {
    pub fn new(slice: TypeSliceFactory<T, F>, reader: TypeReaderFactory<T, F>) -> Self {
        Self { slice, reader }
    }
}

pub struct TypeIdRegistry<M: KeyedHandle<Arc<str>, TypeId>, R: IndexedHandle<Arc<str>>> {
    id_map: M,
    registry: R,
}

impl<M: KeyedHandle<Arc<str>, TypeId>, R: IndexedHandle<Arc<str>>> TypeIdRegistry<M, R>
where
    R::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
    <R::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    <R::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn new(id_map: M, registry: R) -> Self {
        Self { id_map, registry }
    }

    pub fn register<U: TypeName>(&self) -> Result<TypeId, HandleError> {
        self.register_named(U::type_with_generics())
    }

    pub fn register_named(&self, name: impl AsRef<str>) -> Result<TypeId, HandleError> {
        let name = name.as_ref();
        let id = match self.id_map.get(name)? {
            Some(id) => id,
            None => {
                let id = self
                    .registry
                    .push(name.into())?
                    .try_into()
                    .map_err(|e| HandleError::ConversionFailed(Box::new(e)))?;
                self.id_map.insert(name.into(), id)?;
                id
            }
        };
        Ok(id)
    }

    pub fn get_id<U: TypeName>(&self) -> Result<Option<TypeId>, HandleError> {
        self.get_id_by_name(&U::type_with_generics())
    }

    pub fn get_id_by_name(&self, name: impl AsRef<str>) -> Result<Option<TypeId>, HandleError> {
        self.id_map.get(name.as_ref())
    }

    pub fn get_name_by_id(&self, id: TypeId) -> Result<Option<Arc<str>>, HandleError> {
        self.registry
            .get(&R::Key::try_from(id).map_err(|e| HandleError::ConversionFailed(Box::new(e)))?)
    }
}

//TODO: Add doc comments and examples
#[macro_export]
macro_rules! init_format_registry {
    ($NAME:ident: $T:ty, $K:ty, $I:ty) => {
        $crate::paste! {
            $crate::global_thread_local! {//REVIEW: Should this be `FormatId` instead of I::Key
                pub static [< $NAME _FORMATS>]: $crate::serde_utils::serde_registries::FormatRegistry<$T, $I<$crate::serde_utils::serde_format::Format<$T>>, $K<String, <$I<$crate::serde_utils::serde_format::Format<$T>> as $crate::collections::storage::utils::indexed::IndexedHandleRead<$crate::serde_utils::serde_format::Format<$T>>>::Key>> = {
                    $crate::serde_utils::serde_registries::FormatRegistry::new(Default::default(), Default::default())
                };
            }
        }
    };
}

//TODO: Add doc comments and examples
#[macro_export]
macro_rules! init_type_registries {
    ($NAME:ident: $T:ty, $K:ty, $I:ty, $M:ty, $S:ty) => {
        $crate::paste! {
            $crate::global_thread_local! {
                pub static [< $NAME _TYPE_IDS>]: $crate::serde_utils::serde_registries::TypeIdRegistry<$K<std::sync::Arc<str>, $crate::serde_utils::serde_registries::TypeId>, $I<std::sync::Arc<str>>> = {
                    $crate::serde_utils::serde_registries::TypeIdRegistry::new(Default::default(), Default::default())
                };
            }
            $crate::global_thread_local! {
                pub static [< $NAME _TYPE_REGISTRY>]: $crate::serde_utils::serde_registries::FormatTypeRegistry<$T, &'static $crate::serde_utils::serde_registries::TypeIdRegistry<$K<std::sync::Arc<str>, $crate::serde_utils::serde_registries::TypeId>, $I<std::sync::Arc<str>>>, $K<std::sync::Arc<str>, $crate::serde_utils::serde_registries::TypeId>, $I<std::sync::Arc<str>>, $M<$crate::serde_utils::serde_registries::TypeId, <$S<$crate::serde_utils::serde_registries::SerdeFactory<$T>> as $crate::collections::storage::utils::indexed::IndexedHandleRead<$crate::serde_utils::serde_registries::SerdeFactory<$T>>>::Key>, $S<$crate::serde_utils::serde_registries::SerdeFactory<$T>>> = {
                    $crate::serde_utils::serde_registries::FormatTypeRegistry::new([< $NAME _TYPE_IDS>](), Default::default(), Default::default())
                };
            }
        }
    };
}

//TODO: Add doc comments and examples
#[macro_export]
macro_rules! init_registries {
    ($NAME:ident: $T:ty, $K:ty, $I:ty, $M:ty, $S:ty) => {
        $crate::init_format_registry!($NAME: $T, $K, $I);
        $crate::init_type_registries!($NAME: $T, $K, $I, $M, $S);
    };
}

#[cfg(test)]
mod tests {
    use std::{any::Any, collections::HashMap};

    use super::*;
    use crate::{
        collections::storage::RwLockStorage,
        serde_utils::{
            formats::{BinaryFormat, JsonFormat},
            serde_format::SerializeFormat,
        },
    };
    use al_derive::TypeName;
    use serde::{Deserialize, Serialize};

    type RwLockHashMap<K, V> = RwLockStorage<HashMap<K, V>>;
    type RwLockVec<V> = RwLockStorage<Vec<V>>;
    init_registries!(TEST: Test, RwLockHashMap, RwLockVec, RwLockHashMap, RwLockVec);

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypeName)]
    pub enum Test {
        A,
        B(u8),
        C { name: String, age: u8 },
    }

    #[test]
    fn concrete_registries() {
        let test = Test::C {
            name: "Grace".into(),
            age: 42,
        };

        let json_format_id = TEST_FORMATS().register(JsonFormat).unwrap();
        let json_format = TEST_FORMATS().get_format(json_format_id).unwrap().unwrap();

        let mut json_encoded = Vec::new();
        json_format.serialize(&test, &mut json_encoded).unwrap();

        let type_id = TEST_TYPE_REGISTRY().register::<Test>(|u| u).unwrap();

        let json_decoded = TEST_TYPE_REGISTRY()
            .deserialize_slice(&json_format, type_id, &json_encoded)
            .unwrap();
        assert_eq!(json_decoded, test);
        assert!(TEST_FORMATS()
            .get_format_by_name(<JsonFormat as TypeName>::type_with_generics())
            .unwrap()
            .is_some());
        assert!(TEST_TYPE_REGISTRY()
            .get_serde_factory_by_name(<Test as TypeName>::type_with_generics())
            .unwrap()
            .is_some());

        let b_fmt = BinaryFormat::new(RwLockStorage::new(HashMap::new()));
        let binary_format_name = b_fmt.type_with_generics();
        let binary_type_id = b_fmt
            .register::<Test, _, _, _>(TEST_TYPE_IDS(), |u| u)
            .unwrap();
        assert_eq!(binary_type_id, type_id);
        let binary_format_id = TEST_FORMATS().register(b_fmt).unwrap();
        let binary_format = TEST_FORMATS()
            .get_format(binary_format_id)
            .unwrap()
            .unwrap();

        let mut binary_encoded = Vec::new();
        binary_format.serialize(&test, &mut binary_encoded).unwrap();

        let binary_decoded = TEST_TYPE_REGISTRY()
            .deserialize_slice(&binary_format, type_id, &binary_encoded)
            .unwrap();
        assert_eq!(binary_decoded, test);
        assert!(TEST_FORMATS()
            .get_format_by_name(binary_format_name)
            .unwrap()
            .is_some());
    }

    #[test]
    fn any_registries() {
        let test = Test::C {
            name: "Grace".into(),
            age: 42,
        };

        let id_reg = TypeIdRegistry::new(
            RwLockStorage::new(HashMap::new()),
            RwLockStorage::new(Vec::new()),
        );
        let type_reg = FormatTypeRegistry::<Box<dyn Any>, _, _, _, _, _>::new(
            &id_reg,
            RwLockStorage::new(HashMap::new()),
            RwLockStorage::new(Vec::new()),
        );
        let format_reg = FormatRegistry::<Box<dyn Any>, _, _>::new(
            RwLockStorage::new(Vec::new()),
            RwLockStorage::new(HashMap::new()),
        );

        let json_format_id = format_reg.register(JsonFormat).unwrap();
        let json_format = format_reg.get_format(json_format_id).unwrap().unwrap();

        let mut json_encoded = Vec::new();
        json_format.serialize(&test, &mut json_encoded).unwrap();

        let type_id = type_reg.register(|u: Test| Box::new(u)).unwrap();

        let json_decoded = type_reg
            .deserialize_slice(&json_format, type_id, &json_encoded)
            .unwrap();
        assert_eq!(*json_decoded.downcast::<Test>().unwrap(), test);
        assert!(format_reg
            .get_format_by_name(<JsonFormat as TypeName>::type_with_generics())
            .unwrap()
            .is_some());
        assert!(type_reg
            .get_serde_factory_by_name(<Test as TypeName>::type_with_generics())
            .unwrap()
            .is_some());

        let b_fmt = BinaryFormat::<Box<dyn Any>, _>::new(RwLockStorage::new(HashMap::new()));
        let binary_format_name = b_fmt.type_with_generics();
        let binary_type_id = b_fmt.register(&id_reg, |u: Test| Box::new(u)).unwrap();
        assert_eq!(binary_type_id, type_id);
        let binary_format_id = format_reg.register(b_fmt).unwrap();
        let binary_format = format_reg.get_format(binary_format_id).unwrap().unwrap();

        let mut binary_encoded = Vec::new();
        binary_format.serialize(&test, &mut binary_encoded).unwrap();

        let binary_decoded = type_reg
            .deserialize_slice(&binary_format, type_id, &binary_encoded)
            .unwrap();
        assert_eq!(*binary_decoded.downcast::<Test>().unwrap(), test);
        assert!(format_reg
            .get_format_by_name(binary_format_name)
            .unwrap()
            .is_some());
    }
}
