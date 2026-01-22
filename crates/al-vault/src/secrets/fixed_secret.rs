use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    container::secure_container::SecureAccess, AsSecurityLevel, Ephemeral, SecureContainer,
    SecureRef,
};
use al_crypto::fill_random;
use secrets::{Secret, SecretBox};
use xutex::AsyncMutex;

/// For raw, fixed-size byte arrays.
/// This is the most efficient and secure for keys, tokens, etc. when the size is known at compile time.
/// It uses `secrets::SecretBox<T>` directly.
pub struct FixedSecret<const N: usize, L: AsSecurityLevel = Ephemeral> {
    inner: AsyncMutex<SecretBox<[u8; N]>>,
    tag: String,
    access_count: AtomicU64,
    _phantom: PhantomData<L>,
}

impl<const N: usize, L: AsSecurityLevel> FixedSecret<N, L> {
    pub fn random(tag: impl Into<String>) -> Self {
        Self {
            inner: AsyncMutex::new(SecretBox::<[u8; N]>::new(|s| {
                if let Err(_) = fill_random(s) {
                    Secret::<[u8; N]>::random(|bytes| s.copy_from_slice(&*bytes))
                }
            })),
            tag: tag.into(),
            access_count: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }

    /// Will consume the data in `inner`, zeroing it before dropping
    pub fn new(mut inner: [u8; N], tag: impl Into<String>) -> Self {
        Self::take(&mut inner, tag)
    }

    /// Will zero out the data in `inner` after taking it
    pub fn take(inner: &mut [u8; N], tag: impl Into<String>) -> Self {
        Self {
            // `SecretBox::from` will attempt to zero out the data in `inner` after taking it
            inner: AsyncMutex::new(SecretBox::from(inner)),
            tag: tag.into(),
            access_count: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, L: AsSecurityLevel> SecureContainer for FixedSecret<N, L> {
    type InnerType = [u8; N];
    type SecurityLevel = L;

    fn tag(&self) -> &str {
        &self.tag
    }

    fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::SeqCst)
    }
}

impl<const N: usize, L: AsSecurityLevel> SecureAccess for FixedSecret<N, L> {
    type ResultType<R> = R;

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        f(SecureRef::new(*self.inner.lock_sync().borrow()).get())
    }

    async fn with_async<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        f(SecureRef::new(*self.inner.lock().await.borrow()).get())
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
        let mut secure_ref = SecureRef::new(*guard.borrow());
        let result = f(secure_ref.get_mut());
        guard.borrow_mut().copy_from_slice(secure_ref.get());
        result
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
        let mut secure_ref = SecureRef::new(*guard.borrow());
        let result = f(secure_ref.get_mut());
        guard.borrow_mut().copy_from_slice(secure_ref.get());
        result
    }
}

/*impl<const N: usize> EncryptedExt for FixedSecret<N, Encrypted> {
    type EphemeralType = FixedSecret<N, Ephemeral>;

    fn to_ephemeral(self) -> Self::EphemeralType {
        FixedSecret::<N, Ephemeral>::new(*self.inner.borrow(), self.tag)
    }
}*/

#[cfg(test)]
mod tests {
    use crate::{container::secure_container::ToSecureContainer, FixedSecret};

    #[test]
    fn secure_container() {
        let secret = FixedSecret::<32>::random("Test Secret");
        let _container = secret.as_container();
        let _container = secret.to_box();
    }
}
