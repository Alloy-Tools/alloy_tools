use al_vault::{FixedSecret, SecureAccess};
use zeroize::Zeroize;
use crate::{DHLEN, NoiseError};

/// Holds the private key bytes in protected memory
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyPair(FixedSecret<DHLEN>, PublicKey);

impl KeyPair {
    const KEY_PAIR_TAG: &str = "Local Private DH Key";
    pub fn new() -> Self {
        let private = FixedSecret::random(Self::KEY_PAIR_TAG);
        let public = private.with(|private| {
            let mut secret = x25519_dalek::StaticSecret::from(*private);
            let public = PublicKey::new(&secret);
            secret.zeroize();
            public
        });
        Self(private, public)
    }

    pub fn from_bytes(private: [u8; DHLEN]) -> Self {
        let mut secret = x25519_dalek::StaticSecret::from(private);
        let private = FixedSecret::new(secret.to_bytes(), Self::KEY_PAIR_TAG);
        let public = PublicKey::new(&secret);
        secret.zeroize();
        Self(private, public)
    }

    pub fn public(&self) -> &PublicKey {
        &self.1
    }

    #[allow(unused)]
    pub(crate) fn private(&self) -> &FixedSecret<DHLEN> {
        &self.0
    }

    pub fn diffie_hellman(&self, remote_public: [u8; DHLEN]) -> [u8; DHLEN] {
        self.0.with(|private| {
            al_crypto::diffie_hellman(*private, remote_public)
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKey(x25519_dalek::PublicKey);

impl PublicKey {
    pub fn new(public: impl Into<x25519_dalek::PublicKey>) -> Self {
        Self(public.into())
    }

    pub fn from_bytes(public: &[u8]) -> Result<Self, NoiseError> {
        Ok(Self(x25519_dalek::PublicKey::from(<[u8; DHLEN]>::try_from(public).map_err(|_| NoiseError::InvalidKeyLength)?)))
    }

    pub fn to_bytes(&self) -> [u8; DHLEN] {
        self.0.to_bytes()
    }

    pub fn as_bytes(&self) -> &[u8; DHLEN] {
        self.0.as_bytes()
    }
}
