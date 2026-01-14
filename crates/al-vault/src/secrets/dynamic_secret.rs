use crate::{AsSecurityLevel, SecureContainer, SecureRef, Secureable};
use al_crypto::fill_random;
use secrets::SecretVec;
use std::marker::PhantomData;

/// For complex, serializable types (String, Vec, structs).
/// Stores them as a `secrets::SecretVec<u8>` of their serialized form.
pub struct DynamicSecret<T: Secureable, L: AsSecurityLevel> {
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
}

impl<T: Secureable, L: AsSecurityLevel> SecureContainer<L> for DynamicSecret<T, L> {
    type InnerType = T;
    type OutputType = Result<Self, bitcode::Error>;
    type ResultType<R> = Result<R, bitcode::Error>;

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
        let mut bytes = inner.to_bytes()?;
        inner.zeroize();
        Ok(Self {
            inner: SecretVec::from(bytes.as_mut_slice()),
            tag: tag.into(),
            _phantom: PhantomData,
        })
    }

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        self.audit_access("access");
        Ok(f(SecureRef::to_type(&self.inner.borrow())?.get()))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        self.audit_access("mutable access");
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
