use std::sync::Arc;

use erased_serde::{Deserializer, Error};

type Result<T> = std::result::Result<T, Error>;

pub struct DeserializeFromDeFn<T>(
    Arc<dyn for<'de> Fn(&mut dyn Deserializer<'de>) -> Result<T> + Send + Sync>,
);

impl<T> Clone for DeserializeFromDeFn<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> DeserializeFromDeFn<T> {
    pub fn new(
        f: impl for<'de> Fn(&mut dyn Deserializer<'de>) -> Result<T> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(f))
    }

    pub fn call<'de>(&self, de: &mut dyn Deserializer<'de>) -> Result<T> {
        self.0(de)
    }
}

impl<T> std::ops::Deref for DeserializeFromDeFn<T> {
    type Target = dyn for<'de> Fn(&mut dyn Deserializer<'de>) -> Result<T> + Send + Sync;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct DeserializeFromBytesFn<T>(Arc<dyn Fn(&[u8]) -> Result<T> + Send + Sync>);

impl<T> Clone for DeserializeFromBytesFn<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> DeserializeFromBytesFn<T> {
    pub fn new(f: impl Fn(&[u8]) -> Result<T> + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    pub fn call(&self, bytes: &[u8]) -> Result<T> {
        self.0(bytes)
    }
}

impl<T> std::ops::Deref for DeserializeFromBytesFn<T> {
    type Target = dyn Fn(&[u8]) -> Result<T> + Send + Sync;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
