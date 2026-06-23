use crate::DeserializerFn;
use al_structures::collections::registries::RegistryRead;
use serde::de::{DeserializeSeed, Visitor};
use std::marker::PhantomData;

/// A generic serde visitor that works with any registry implementing `RegistryRead`.
/// `T` is the trait object produced (e.g., `dyn Command`).
pub struct GenericMessageVisitor<'a, R, T: ?Sized> {
    pub registry: &'a R,
    _marker: PhantomData<T>,
}

impl<'a, R, T: ?Sized> GenericMessageVisitor<'a, R, T> {
    pub fn new(registry: &'a R) -> Self {
        Self {
            registry,
            _marker: PhantomData,
        }
    }
}

impl<'de, R, T> Visitor<'de> for GenericMessageVisitor<'_, R, T>
where
    T: ?Sized + 'static,
    R: RegistryRead<String, DeserializerFn<Box<T>>> + 'static,
{
    type Value = Box<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple (type name, data)")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let type_name: String = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("expected type name"))?;

        Ok(seq
            .next_element_seed(GenericMessageSeed {
                type_name: &type_name,
                registry: self.registry,
                _marker: PhantomData,
            })?
            .ok_or_else(|| serde::de::Error::custom("expected message data"))?)
    }
}

/// Seed for deserializing a specific message type using its type name and the registry
struct GenericMessageSeed<'a, R, T: ?Sized> {
    type_name: &'a str,
    registry: &'a R,
    _marker: PhantomData<T>,
}

impl<'de, R, T> DeserializeSeed<'de> for GenericMessageSeed<'_, R, T>
where
    T: ?Sized + 'static,
    R: RegistryRead<String, DeserializerFn<Box<T>>>,
{
    type Value = Box<T>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let deser = self
            .registry
            .get(self.type_name)
            .map_err(|e| serde::de::Error::custom(format!("Registry get error: {e}")))?
            .ok_or_else(|| serde::de::Error::custom(format!("unknown type: {}", self.type_name)))?;

        deser
            .call(&mut <dyn erased_serde::Deserializer>::erase(deserializer))
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}
