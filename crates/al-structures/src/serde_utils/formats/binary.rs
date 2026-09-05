use std::{ops::Deref, sync::Arc};

use al_derive::TypeName;

use crate::{
    collections::storage::utils::keyed::KeyedHandle,
    serde_utils::{
        serde_format::{DeserializeInto, ErasedDeserialize, Format, SerializeFormat},
        serde_registries::{DirectFactory, TypeId, TypeIdRegistry},
    },
};

//REVIEW: Also add a `bincode` format for performance testing
#[derive(Copy, Default, PartialEq, Eq, Hash, TypeName)]
pub struct BinaryFormat<T, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync> {
    deserializers: D,
    _phantom: std::marker::PhantomData<fn() -> T>,
}

impl<T, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync> BinaryFormat<T, D> {
    pub fn new(deserializers: D) -> Self {
        Self {
            deserializers,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync> Clone
    for BinaryFormat<T, D>
{
    fn clone(&self) -> Self {
        Self {
            deserializers: self.deserializers.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync> std::fmt::Debug
    for BinaryFormat<T, D>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinaryFormat")
            .field("deserializers", &self.deserializers.len())
            .finish()
    }
}

impl<T: 'static, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync + 'static>
    From<BinaryFormat<T, D>> for Format<T>
{
    fn from(value: BinaryFormat<T, D>) -> Self {
        Self::Erased(Box::new(value))
    }
}

impl<T, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync> SerializeFormat
    for BinaryFormat<T, D>
{
    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize(
        &self,
        value: &dyn erased_serde::Serialize,
        writer: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(writer.write_all(&bitcode::serialize(value)?)?)
    }
}

impl<_T, D: KeyedHandle<TypeId, DirectFactory<_T>> + Clone + Send + Sync> DeserializeInto
    for BinaryFormat<_T, D>
{
    fn deserialize_slice_into<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
    ) -> Result<T, Box<dyn std::error::Error>> {
        Ok(bitcode::deserialize(data)?)
    }

    fn deserialize_reader_into<T: for<'de> serde::de::Deserialize<'de>>(
        &self,
        reader: &mut dyn std::io::Read,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        let len = reader.read_to_end(&mut bytes)?;
        Ok(bitcode::deserialize(&bytes[..len])?)
    }
}

impl<T: 'static, D: KeyedHandle<TypeId, DirectFactory<T>> + Clone + Send + Sync + 'static>
    ErasedDeserialize<T> for BinaryFormat<T, D>
{
    fn clone_format(&self) -> Box<dyn ErasedDeserialize<T>> {
        Box::new(self.clone())
    }

    fn register_named<
        U: for<'de> serde::Deserialize<'de>,
        R: Deref<Target = TypeIdRegistry<K, I>>,
        K: KeyedHandle<Arc<str>, TypeId>,
        I: crate::collections::storage::utils::indexed::IndexedHandle<Arc<str>>,
    >(
        &self,
        name: impl AsRef<str>,
        type_registry: R,
        into_target: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> Result<TypeId, Box<dyn std::error::Error>>
    where
        Self: Sized,
        I::Key: TryInto<TypeId> + TryFrom<TypeId> + Eq,
        <I::Key as TryFrom<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
        <I::Key as TryInto<TypeId>>::Error: std::error::Error + Send + Sync + 'static,
    {
        crate::register_type!(self, type_registry, into_target, name, BinaryFormat<T, D>, "Failed to downcast to BinaryFormat")
    }

    fn get_deserializer(
        &self,
        type_id: TypeId,
    ) -> Result<Option<DirectFactory<T>>, Box<dyn std::error::Error>> {
        Ok(self.deserializers.get(&type_id)?.clone())
    }
}
