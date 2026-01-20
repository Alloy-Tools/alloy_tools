use std::marker::PhantomData;

use crate::{
    container::secure_container::{SecureAccess, ToSecureContainer},
    AsSecurityLevel, Ephemeral, SecureContainer, SecureRef,
};
use al_crypto::fill_random;
use secrets::{Secret, SecretBox};

/// For raw, fixed-size byte arrays.
/// This is the most efficient and secure for keys, tokens, etc. when the size is known at compile time.
/// It uses `secrets::SecretBox<T>` directly.
pub struct FixedSecret<const N: usize, L: AsSecurityLevel = Ephemeral> {
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

    /// Will consume the data in `inner`, zeroing it before dropping
    pub fn new(mut inner: [u8; N], tag: impl Into<String>) -> Self {
        Self::take(&mut inner, tag)
    }

    /// Will zero out the data in `inner` after taking it
    pub fn take(inner: &mut [u8; N], tag: impl Into<String>) -> Self {
        Self {
            // `SecretBox::from` will attempt to zero out the data in `inner` after taking it
            inner: SecretBox::from(inner),
            tag: tag.into(),
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, L: AsSecurityLevel> SecureContainer for FixedSecret<N, L> {
    type SecretType = Self;
    type InnerType = [u8; N];
    type SecurityLevel = L;

    fn tag(&self) -> &str {
        &self.tag
    }

    fn access(&self) -> &Self::SecretType {
        self
    }
}

impl<'a, const N: usize, L: AsSecurityLevel> From<&'a FixedSecret<N, L>>
    for &'a dyn SecureContainer<
        SecretType = FixedSecret<N, L>,
        InnerType = [u8; N],
        SecurityLevel = L,
    >
{
    fn from(value: &'a FixedSecret<N, L>) -> Self {
        value
    }
}

impl<const N: usize, L: AsSecurityLevel> ToSecureContainer for FixedSecret<N, L> {}

impl<const N: usize, L: AsSecurityLevel> SecureAccess for FixedSecret<N, L> {
    type ResultType<R> = R;

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

/*impl<const N: usize> EncryptedExt for FixedSecret<N, Encrypted> {
    type EphemeralType = FixedSecret<N, Ephemeral>;

    fn to_ephemeral(self) -> Self::EphemeralType {
        FixedSecret::<N, Ephemeral>::new(*self.inner.borrow(), self.tag)
    }
}*/

#[cfg(test)]
mod tests {
    use crate::{FixedSecret, SecureContainer, container::secure_container::ToSecureContainer};

    #[test]
    fn secure_container() {
        let secret = FixedSecret::<32>::random("Test Secret");
        let container = secret.to_box();
    }
}
