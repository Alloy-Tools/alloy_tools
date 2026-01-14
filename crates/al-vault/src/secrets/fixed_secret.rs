use std::marker::PhantomData;

use crate::{AsSecurityLevel, SecureContainer, SecureRef};
use al_crypto::fill_random;
use secrets::{Secret, SecretBox};

/// For raw, fixed-size byte arrays.
/// This is the most efficient and secure for keys, tokens, etc. when the size is known at compile time.
/// It uses `secrets::SecretBox<T>` directly.
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
    type ResultType<R> = R;

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

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R> {
        self.audit_access("access");
        f(SecureRef::new(*self.inner.borrow()).get())
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R> {
        self.audit_access("mutable access");
        let mut secure_ref = SecureRef::new(*self.inner.borrow());
        let result = f(secure_ref.get_mut());
        self.inner.borrow_mut().copy_from_slice(secure_ref.get());
        result
    }
}
