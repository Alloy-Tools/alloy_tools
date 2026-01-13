use std::marker::PhantomData;

use al_crypto::fill_random;
use secrets::{Secret, SecretBox};

mod keys;
//mod system_control;

mod sealed {
    pub trait Sealed {}
}

pub trait Secureable:
    serde::Serialize + for<'de> serde::Deserialize<'de> + zeroize::Zeroize
{
    fn to_bytes(&self) -> Result<Vec<u8>, bitcode::Error>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, bitcode::Error>;
}
impl<T: serde::Serialize + for<'de> serde::Deserialize<'de> + zeroize::Zeroize> Secureable for T {
    fn to_bytes(&self) -> Result<Vec<u8>, bitcode::Error> {
        bitcode::serialize(self)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, bitcode::Error> {
        bitcode::deserialize(bytes)
    }
}

/******
 * Security Level
***** */
pub trait AsSecurityLevel: sealed::Sealed {
    fn as_security_level() -> SecurityLevel;
}
#[derive(Debug, Clone, Copy)]
pub struct Ephemeral;
impl sealed::Sealed for Ephemeral {}
impl AsSecurityLevel for Ephemeral {
    fn as_security_level() -> SecurityLevel {
        SecurityLevel::Ephemeral
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Encrypted;
impl sealed::Sealed for Encrypted {}
impl AsSecurityLevel for Encrypted {
    fn as_security_level() -> SecurityLevel {
        SecurityLevel::Encrypted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    Ephemeral,
    Encrypted,
}

impl SecurityLevel {
    pub fn is_ephemeral(&self) -> bool {
        matches!(self, SecurityLevel::Ephemeral)
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, SecurityLevel::Encrypted)
    }

    pub fn is_persistable(&self) -> bool {
        self.is_encrypted()
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            SecurityLevel::Ephemeral => "ephemeral",
            SecurityLevel::Encrypted => "encrypted",
        }
    }
}

pub trait SecureContainer<L: AsSecurityLevel> {
    type InnerType;
    type OutputType;
    fn tag(&self) -> &str;
    fn audit_access(&self, operation: &str);
    /// Will consume the data in `inner`, zeroing it before dropping
    fn new(inner: Self::InnerType, tag: impl Into<String>) -> Self::OutputType;
    /// Will zero out the data in `inner` after taking it
    fn take(inner: &mut Self::InnerType, tag: impl Into<String>) -> Self::OutputType;
    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Result<R, bitcode::Error>;
    fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Self::InnerType) -> R,
    ) -> Result<R, bitcode::Error>;

    fn security_level() -> SecurityLevel {
        L::as_security_level()
    }
}

/* ********************
  FixedSecret
******************** */
pub struct FixedSecret<const N: usize, L: AsSecurityLevel> {
    inner: SecretBox<[u8; N]>,
    tag: String,
    _phantom: PhantomData<L>,
}

impl<const N: usize, L: AsSecurityLevel> FixedSecret<N, L> {
    pub fn random(tag: impl Into<String>) -> Self {
        Self {
            inner: SecretBox::<[u8; N]>::new(|s| {
                if let Err(_) = fill_random(s) {
                    Secret::<[u8; N]>::random(|bytes| s.copy_from_slice(&*bytes))
                }
            }),
            tag: tag.into(),
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, L: AsSecurityLevel> SecureContainer<L> for FixedSecret<N, L> {
    type InnerType = [u8; N];
    type OutputType = FixedSecret<N, L>;

    fn tag(&self) -> &str {
        &self.tag
    }

    fn audit_access(&self, _operation: &str) {
        todo!()
        //lazy static AuditLog: Vec<AuditEntry> using audit_log!() macro
    }

    fn new(mut inner: Self::InnerType, tag: impl Into<String>) -> Self::OutputType {
        Self::take(&mut inner, tag)
    }

    fn take(inner: &mut Self::InnerType, tag: impl Into<String>) -> Self::OutputType {
        Self {
            // `SecretBox::from` will attempt to zero out the data in `inner` after taking it
            inner: SecretBox::from(inner),
            tag: tag.into(),
            _phantom: PhantomData,
        }
    }

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Result<R, bitcode::Error> {
        self.audit_access("access");
        Ok(f(SecureRef::new(*self.inner.borrow()).get()))
    }

    fn with_mut<R>(
        &mut self,
        f: impl FnOnce(&mut Self::InnerType) -> R,
    ) -> Result<R, bitcode::Error> {
        self.audit_access("mutable access");
        let mut secure_ref = SecureRef::new(*self.inner.borrow());

        let bytes = secure_ref.get_mut();
        let result = f(bytes);
        {
            let mut dest = self.inner.borrow_mut();
            dest.copy_from_slice(bytes);
        }

        Ok(result)
    }
}

/* ********************
  SecureRef
******************** */
#[derive(zeroize::ZeroizeOnDrop)]
pub struct SecureRef<'a, T: zeroize::Zeroize> {
    value: T,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, T: zeroize::Zeroize> SecureRef<'a, T> {
    fn new(value: T) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }

    fn get(&self) -> &T {
        &self.value
    }

    fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'a, T: Secureable> SecureRef<'a, T> {
    /// Won't zeroize the passed `&[u8]`
    fn to_type(bytes: &'a [u8]) -> Result<Self, bitcode::Error> {
        Ok(Self {
            value: T::from_bytes(bytes)?,
            _phantom: PhantomData,
        })
    }
}
// *********************************************

/*/// For raw bytes and fixed-size, `Bytes`-compatible types.
/// This is the most efficient and secure for keys, tokens, etc.
/// It uses `secrets::Secret<T>` directly.
pub struct SecureBytes<T: SecureableBytes, L: AsSecurityLevel> {
    inner: secrets::Secret<T>,
    tag: String,
    _phantom: PhantomData<L>,
}

/// For complex, serializable types (String, Vec, structs).
/// Stores them as a `secrets::SecretVec<u8>` of their serialized form.
pub struct SecureValue<T: Secureable, L: AsSecurityLevel> {
    inner: secrets::SecretVec<u8>,
    tag: String,
    _phantom: PhantomData<(T, L)>,
}

impl<T: Secureable, L: AsSecurityLevel> SecureContainer<L> for SecureValue<T, L> {
    type InnerType = T;
    type OutputType = Result<Self, bitcode::Error>;

    fn tag(&self) -> &str {
        &self.tag
    }

    fn audit_access(&self, _operation: &str) {
        todo!()
        //lazy static AuditLog: Vec<AuditEntry> using audit_log!() macro
    }

    fn take(inner: Self::InnerType, tag: impl Into<String>) -> Self::OutputType {
        SecureValue::new(inner, tag)
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureValue<T, L> {
    pub fn new(mut value: T, tag: impl Into<String>) -> Result<Self, bitcode::Error> {
        let mut bytes = value.to_bytes()?;
        value.zeroize();
        Ok(Self {
            inner: secrets::SecretVec::from(bytes.as_mut_slice()),
            tag: tag.into(),
            _phantom: PhantomData,
        })
    }

    /// Access the deserialized value
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, bitcode::Error> {
        self.audit_access("access");
        let bytes = self.inner.borrow();
        let mut value: T = T::from_bytes(&bytes)?;
        let result = f(&value);
        value.zeroize();
        Ok(result)
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Result<R, bitcode::Error> {
        self.audit_access("mutable access");
        let mut value: T = { T::from_bytes(&self.inner.borrow_mut())? };

        let result = f(&mut value);

        let mut new_bytes = value.to_bytes()?;
        let len = new_bytes.len();
        if self.inner.len() == len {
            let mut dest = self.inner.borrow_mut();
            dest.copy_from_slice(&new_bytes);
        } else {
            self.inner = secrets::SecretVec::new(len, |s| s.copy_from_slice(&new_bytes));
        }

        new_bytes.zeroize();
        value.zeroize();

        Ok(result)
    }
}*/

#[cfg(test)]
mod tests {

    #[test]
    fn todo() {}
}
