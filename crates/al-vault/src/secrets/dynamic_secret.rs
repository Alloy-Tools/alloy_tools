use crate::{
    container::secure_container::SecureAccess, AsSecurityLevel, Ephemeral, SecureContainer,
    SecureRef, Secureable,
};
use al_crypto::fill_random;
use secrets::SecretVec;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};
use xutex::AsyncMutex;

/// For complex, serializable types (String, Vec, structs).
/// Stores them as a `secrets::SecretVec<u8>` of their serialized form.
pub struct DynamicSecret<T: Secureable, L: AsSecurityLevel = Ephemeral> {
    inner: AsyncMutex<SecretVec<u8>>,
    tag: String,
    access_count: AtomicU64,
    _phantom: PhantomData<(T, L)>,
}

impl<T: Secureable, L: AsSecurityLevel> DynamicSecret<T, L> {
    pub fn random(tag: impl Into<String>, len: usize) -> Self {
        Self {
            inner: AsyncMutex::new(SecretVec::<u8>::new(len, |s| {
                if let Err(_) = fill_random(s) {
                    s.copy_from_slice(&SecretVec::<u8>::random(len).borrow());
                }
            })),
            tag: tag.into(),
            access_count: AtomicU64::new(0),
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
            inner: AsyncMutex::new(SecretVec::from(bytes.as_mut_slice())),
            tag: tag.into(),
            access_count: AtomicU64::new(0),
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

    fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::SeqCst)
    }
}

impl<T: Secureable, L: AsSecurityLevel> SecureAccess for DynamicSecret<T, L> {
    type ResultType<R> = Result<R, bitcode::Error>;

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        Ok(f(
            SecureRef::to_type(&self.inner.lock_sync().borrow())?.get()
        ))
    }

    async fn with_async<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        Ok(f(
            SecureRef::to_type(&self.inner.lock().await.borrow())?.get()
        ))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "mutable access",
        );
        let mut guard = self.inner.lock_sync();
        let mut secure_ref = SecureRef::to_type(&guard.borrow())?;

        let result = f(secure_ref.get_mut());
        let secure_bytes = secure_ref.to_bytes()?;
        let new_len = secure_bytes.get().len();

        //TODO: test more with inner.len(), could maybe do if inner.len() >= len and only reallocate when needed?
        if guard.len() == new_len {
            guard.borrow_mut().copy_from_slice(secure_bytes.get());
        } else {
            *guard = SecretVec::new(new_len, |s| s.copy_from_slice(secure_bytes.get()))
        }

        Ok(result)
    }

    async fn with_mut_async<R>(
        &mut self,
        f: impl FnOnce(&mut Self::InnerType) -> R,
    ) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "mutable access",
        );
        let mut guard = self.inner.lock().await;
        let mut secure_ref = SecureRef::to_type(&guard.borrow())?;

        let result = f(secure_ref.get_mut());
        let secure_bytes = secure_ref.to_bytes()?;
        let new_len = secure_bytes.get().len();

        //TODO: test more with inner.len(), could maybe do if inner.len() >= len and only reallocate when needed?
        if guard.len() == new_len {
            guard.borrow_mut().copy_from_slice(secure_bytes.get());
        } else {
            *guard = SecretVec::new(new_len, |s| s.copy_from_slice(secure_bytes.get()))
        }

        Ok(result)
    }
}
