use crate::SecretError;

pub trait Secureable:
    serde::Serialize + for<'de> serde::Deserialize<'de> + zeroize::Zeroize + 'static
{
    fn to_bytes(&self) -> Result<Vec<u8>, SecretError> {
        Ok(bitcode::serialize(self)?)
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, SecretError> {
        Ok(bitcode::deserialize(bytes)?)
    }
}
impl<T: serde::Serialize + for<'de> serde::Deserialize<'de> + zeroize::Zeroize + 'static> Secureable
    for T
{
}

pub struct SecureRef<T: zeroize::Zeroize>(T);

impl<T: zeroize::Zeroize> Drop for SecureRef<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// `SecureRef` for any T that is at least zeroizable
impl<T: zeroize::Zeroize> SecureRef<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn get(&self) -> &T {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// `SecureRef` for any type marked `Securable`
impl<T: Secureable> SecureRef<T> {
    /// Won't zeroize the passed `&[u8]`
    pub fn to_type(bytes: &[u8]) -> Result<Self, SecretError> {
        Ok(Self(T::from_bytes(bytes)?))
    }

    /// The generated `Vec` will be zeroized once the `SecureRef` is dropped
    pub fn to_bytes(&self) -> Result<SecureRef<Vec<u8>>, SecretError> {
        Ok(SecureRef::new(self.0.to_bytes()?))
    }
}
