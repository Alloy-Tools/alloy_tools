use std::sync::RwLock;

use crate::{Ephemeral, FixedSecret, SecretError, SecureAccess};
use al_crypto::{decrypt, encrypt, Nonce, NonceTrait, KEY_SIZE};

//TODO: Add rekey checks and alerts

/// Keys contain `FixedSecret` of `Ephemeral` type
#[derive(Debug)]
pub struct Key<T: NonceTrait, const N: usize = KEY_SIZE>(
    FixedSecret<N, Ephemeral>,
    RwLock<Nonce<T>>,
);

impl<T: NonceTrait, const N: usize> Clone for Key<T, N> {
    fn clone(&self) -> Self {
        let nonce = match self.1.read() {
            Ok(nonce) => RwLock::new(nonce.clone()),
            Err(poison) => RwLock::new(poison.into_inner().clone()),
        };
        Self(self.0.clone(), nonce)
    }
}

impl<T: NonceTrait, const N: usize> Eq for Key<T, N> {}
impl<T: NonceTrait, const N: usize> PartialEq for Key<T, N> {
    fn eq(&self, other: &Self) -> bool {
        let local = match self.1.read() {
            Ok(nonce) => nonce,
            Err(poison) => poison.into_inner(),
        };
        let remote = match other.1.read() {
            Ok(nonce) => nonce,
            Err(poison) => poison.into_inner(),
        };
        self.0 == other.0 && *local == *remote
    }
}

impl<T: NonceTrait, const N: usize> Key<T, N> {
    pub fn from_array(value: [u8; N], tag: impl Into<String>, nonce: Nonce<T>) -> Self {
        Self(FixedSecret::new(value, tag), RwLock::new(nonce))
    }

    pub fn from_slice(slice: &mut [u8; N], tag: impl Into<String>, nonce: Nonce<T>) -> Self {
        Self(FixedSecret::take(slice, tag), RwLock::new(nonce))
    }

    pub fn random(tag: impl Into<String>, nonce: Nonce<T>) -> Self {
        Self(FixedSecret::<N, Ephemeral>::random(tag), RwLock::new(nonce))
    }

    pub fn new(key: FixedSecret<N, Ephemeral>, nonce: Nonce<T>) -> Self {
        Self(key, RwLock::new(nonce))
    }

    pub fn encrypt(
        &self,
        dest: &mut [u8],
        plaintext: &[u8],
        nonce: &mut [u8; al_crypto::NONCE_SIZE],
        associated_data: &[u8],
    ) -> Result<(), SecretError> {
        let mut guard = self.1.write()?;
        nonce.copy_from_slice(guard.as_bytes());
        guard.to_next()?;
        drop(guard);
        Ok(self
            .0
            .with(|k| encrypt(dest, plaintext, k, nonce, associated_data))?)
    }

    pub fn decrypt(
        &self,
        dest: &mut [u8],
        ciphertext: &[u8],
        nonce: &[u8; al_crypto::NONCE_SIZE],
        associated_data: &[u8],
    ) -> Result<(), SecretError> {
        Ok(self
            .0
            .with(|k| decrypt(dest, ciphertext, k, nonce, associated_data))?)
    }
}
