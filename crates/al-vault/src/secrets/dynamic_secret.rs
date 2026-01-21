use crate::{
    container::secure_container::SecureAccess, AsSecurityLevel, Ephemeral, SecureContainer,
    SecureRef, Secureable,
};
use al_crypto::fill_random;
use secrets::SecretVec;
use std::marker::PhantomData;

/// For complex, serializable types (String, Vec, structs).
/// Stores them as a `secrets::SecretVec<u8>` of their serialized form.
pub struct DynamicSecret<T: Secureable, L: AsSecurityLevel = Ephemeral> {
    inner: SecretVec<u8>,
    tag: String,
    _phantom: PhantomData<(T, L)>,
}

impl<T: Secureable, L: AsSecurityLevel> DynamicSecret<T, L> {
    pub fn random(tag: impl Into<String>, len: usize) -> Self {
        Self {
            inner: SecretVec::<u8>::new(len, |s| {
                if let Err(_) = fill_random(s) {
                    s.copy_from_slice(&SecretVec::<u8>::random(len).borrow());
                }
            }),
            tag: tag.into(),
            _phantom: PhantomData,
        }
    }

    /// Will consume the data in `inner`, zeroing it before dropping
    pub fn new(mut inner: T, tag: impl Into<String>) -> Result<Self, bitcode::Error> {
        Self::take(&mut inner, tag)
    }

    /// Will zero out the data in `inner` after taking it
    pub fn take(inner: &mut T, tag: impl Into<String>) -> Result<Self, bitcode::Error> {
        let mut bytes = inner.to_bytes()?;
        inner.zeroize();
        Ok(Self {
            inner: SecretVec::from(bytes.as_mut_slice()),
            tag: tag.into(),
            _phantom: PhantomData,
        })
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureContainer for DynamicSecret<T, L> {
    type InnerType = T;
    type SecurityLevel = L;

    fn tag(&self) -> &str {
        &self.tag
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureAccess for DynamicSecret<T, L> {
    type ResultType<R> = Result<R, bitcode::Error>;

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access("access");
        Ok(f(SecureRef::to_type(&self.inner.borrow())?.get()))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access("mutable access");
        let mut secure_ref = SecureRef::to_type(&self.inner.borrow())?;
        let result = f(secure_ref.get_mut());

        let secure_bytes = secure_ref.to_bytes()?;
        let len = secure_bytes.get().len();

        //TODO: test more with inner.len(), could maybe do if inner.len() >= len and only reallocate when needed?
        if self.inner.len() == len {
            self.inner.borrow_mut().copy_from_slice(secure_bytes.get());
        } else {
            self.inner = SecretVec::new(len, |s| s.copy_from_slice(secure_bytes.get()))
        }

        Ok(result)
    }
}
