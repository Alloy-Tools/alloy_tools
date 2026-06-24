use crate::{collections::storage::utils::keyed::KeyedHandleRead, DeserializeFromDeFn};
use std::marker::PhantomData;

//QUESTION: Move this to the crate that ends up using it? OR do the registries bypass it?

/// A serde visitor for any registry implementing `RegistryRead<String, DeserializerFn<T>>`.
/// - `R` is the `RegistryRead`.
/// - `T` is the type produced.
pub struct GenericRegistryVisitor<'a, R, T> {
    pub registry: &'a R,
    _marker: PhantomData<T>,
}

impl<'a, R, T> GenericRegistryVisitor<'a, R, T> {
    pub fn new(registry: &'a R) -> Self {
        Self {
            registry,
            _marker: PhantomData,
        }
    }
}

impl<'de, R, T> serde::de::Visitor<'de> for GenericRegistryVisitor<'_, R, T>
where
    R: KeyedHandleRead<String, DeserializeFromDeFn<T>> + 'static,
{
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple (type_name: String, data: T)")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let type_name: String = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("expected type name"))?;

        Ok(seq
            .next_element_seed(GenericRegistrySeed::new(&type_name, self.registry))?
            .ok_or_else(|| serde::de::Error::custom("expected message data"))?)
    }
}

/// Seed for deserializing a type using its type name and passed registry
struct GenericRegistrySeed<'a, R, T> {
    type_name: &'a str,
    registry: &'a R,
    _marker: PhantomData<T>,
}

impl<'a, R, T> GenericRegistrySeed<'a, R, T> {
    pub fn new(type_name: &'a str, registry: &'a R) -> Self {
        Self {
            type_name,
            registry,
            _marker: PhantomData,
        }
    }
}

impl<'de, R, T> serde::de::DeserializeSeed<'de> for GenericRegistrySeed<'_, R, T>
where
    R: KeyedHandleRead<String, DeserializeFromDeFn<T>>,
{
    type Value = T;

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
