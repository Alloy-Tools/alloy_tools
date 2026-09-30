use crate::{
    container::secure_container::SecureAccess, AsSecurityLevel, Ephemeral, SecureContainer,
    SecureRef,
};
use al_crypto::fill_random;
use secrets::{Secret, SecretBox};
use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

/// For raw, fixed-size byte arrays.
/// This is the most efficient and secure for keys, tokens, etc. when the size is known at compile time.
/// It uses `secrets::SecretBox<T>` directly.
#[derive(Debug)]
pub struct FixedSecret<const N: usize, L: AsSecurityLevel = Ephemeral> {
    inner: Mutex<SecretBox<[u8; N]>>,
    tag: String,
    access_count: AtomicU64,
    _phantom: PhantomData<L>,
}

impl<const N: usize, L: AsSecurityLevel> Clone for FixedSecret<N, L> {
    fn clone(&self) -> Self {
        Self {
            inner: Mutex::new((*self.inner.lock().expect("Secret mutex poisoned")).clone()),
            tag: self.tag.clone(),
            access_count: AtomicU64::new(self.access_count()),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<const N: usize, L: AsSecurityLevel> Eq for FixedSecret<N, L> {}
impl<const N: usize, L: AsSecurityLevel> PartialEq for FixedSecret<N, L> {
    fn eq(&self, other: &Self) -> bool {
        if self.tag == other.tag && self.access_count() == other.access_count() {
            // Lock both mutexes in a consistent order by memory address.
            // This prevents deadlocks when two threads call eq on the same two objects in reverse order.
            let (lower_guard, higher_guard) = if &self.inner as *const _ < &other.inner as *const _
            {
                (
                    self.inner.lock().expect("Secret mutex poisoned"),
                    other.inner.lock().expect("Secret mutex poisoned"),
                )
            } else {
                (
                    other.inner.lock().expect("Secret mutex poisoned"),
                    self.inner.lock().expect("Secret mutex poisoned"),
                )
            };
            *lower_guard == *higher_guard
        } else {
            false
        }
    }
}

impl<const N: usize, L: AsSecurityLevel> FixedSecret<N, L> {
    pub fn random(tag: impl Into<String>) -> Self {
        Self {
            inner: Mutex::new(SecretBox::<[u8; N]>::new(|s| {
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
            inner: Mutex::new(SecretBox::from(inner)),
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

    fn len(&self) -> Result<usize, crate::SecretError> {
        Ok(N)
    }
}

impl<const N: usize, L: AsSecurityLevel> SecureAccess for FixedSecret<N, L> {
    type ResultType<R> = Result<R, crate::SecretError>;
    type CopyResultType = Result<Self::InnerType, crate::SecretError>;

    fn copy(&self) -> Self::CopyResultType {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "copy",
        );
        Ok(*self.inner.lock()?.borrow())
    }

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "access",
        );
        Ok(f(SecureRef::new(*self.inner.lock()?.borrow()).get()))
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        //TODO: handle io error possibility?
        let _ = self.audit_access(
            self.access_count
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1),
            "mutable access",
        );
        let mut guard = self.inner.lock()?;
        let mut secure_ref = SecureRef::new(*guard.borrow());
        let result = f(secure_ref.get_mut());
        guard.borrow_mut().copy_from_slice(secure_ref.get());
        Ok(result)
    }
}
