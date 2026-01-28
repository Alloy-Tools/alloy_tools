use crate::{
    container::secure_container::SecureAccess, AsSecurityLevel, Ephemeral, SecretError,
    SecureContainer, SecureRef, Secureable,
};
use al_crypto::fill_random;
use secrets::SecretVec;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

/// For complex, serializable types (String, Vec, structs).
/// Stores them as a `secrets::SecretVec<u8>` of their serialized form.
#[derive(Debug)]
pub struct DynamicSecret<T: Secureable, L: AsSecurityLevel = Ephemeral> {
    inner: SecretVec<u8>,
    tag: String,
    access_count: AtomicU64,
    _phantom: PhantomData<(T, L)>,
}

impl<T: Secureable, L: AsSecurityLevel> Clone for DynamicSecret<T, L> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            tag: self.tag.clone(),
            access_count: AtomicU64::new(self.access_count()),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<T: Secureable, L: AsSecurityLevel> Eq for DynamicSecret<T, L> {}
impl<T: Secureable, L: AsSecurityLevel> PartialEq for DynamicSecret<T, L> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
            && self.tag == other.tag
            && self.access_count() == other.access_count()
            && self._phantom == other._phantom
    }
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
            access_count: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }

    /// Will consume the data in `inner`, zeroing it before dropping
    pub fn new(mut inner: T, tag: impl Into<String>) -> Result<Self, SecretError> {
        Self::take(&mut inner, tag)
    }

    /// Will zero out the data in `inner` after taking it
    pub fn take(inner: &mut T, tag: impl Into<String>) -> Result<Self, SecretError> {
        let mut bytes = inner.to_bytes()?;
        inner.zeroize();
        Ok(Self {
            inner: SecretVec::from(bytes.as_mut_slice()),
            tag: tag.into(),
            access_count: AtomicU64::new(0),
            _phantom: PhantomData,
        })
    }
}

impl<L: AsSecurityLevel> DynamicSecret<Vec<u8>, L> {
    pub fn inner_len(&self) -> usize {
        if let Ok(s) = SecureRef::<Vec<u8>>::to_type(&self.inner.borrow()) {
            s.get().len()
        } else {
            0
        }
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureContainer for DynamicSecret<T, L> {
    type InnerType = T;
    type SecurityLevel = L;

    fn tag(&self) -> &str {
        &self.tag
    }

    fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::SeqCst)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureAccess for DynamicSecret<T, L> {
    type ResultType<R> = Result<R, SecretError>;
    type CopyResultType = Result<Self::InnerType, SecretError>;

    fn copy(&self) -> Self::CopyResultType {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "copy",
        );
        Ok(SecureRef::<Self::InnerType>::to_type(&self.inner.borrow())?
            .get()
            .clone())
    }

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        Ok(f(SecureRef::to_type(&self.inner.borrow())?.get()))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "mutable access",
        );
        let mut secure_ref = SecureRef::to_type(&self.inner.borrow())?;

        let result = f(secure_ref.get_mut());
        let secure_bytes = secure_ref.to_bytes()?;
        let new_len = secure_bytes.get().len();

        //TODO: test more with inner.len() (or maybe self.len()?), could maybe do if inner.len() >= len and only reallocate when needed?
        if self.inner.len() == new_len {
            self.inner.borrow_mut().copy_from_slice(secure_bytes.get());
        } else {
            self.inner = SecretVec::new(new_len, |s| s.copy_from_slice(secure_bytes.get()))
        }

        Ok(result)
    }
}
