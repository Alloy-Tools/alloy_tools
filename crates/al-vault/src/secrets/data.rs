use al_crypto::{NonceTrait, NONCE_SIZE, TAG_SIZE};

use crate::{DynamicSecret, Key, SecretError, SecureAccess, SecureContainer};

// Marker types
pub trait CryptoState {
    fn to_string() -> String;
}
pub trait ProtectedState {
    fn nonce(&self) -> &[u8; NONCE_SIZE];
    fn from_nonce(nonce: Vec<u8>) -> Result<Self, SecretError>
    where
        Self: Sized;
}
pub trait PlainState: CryptoState {}
pub trait EncryptedState: CryptoState {}
pub trait AuthenticatedState: CryptoState {}

// State types
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Plain;
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Encrypted([u8; NONCE_SIZE]);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authenticated([u8; NONCE_SIZE], Vec<u8>);

impl CryptoState for Plain {
    fn to_string() -> String {
        "<Plain>".to_string()
    }
}
impl PlainState for Plain {}
impl CryptoState for Encrypted {
    fn to_string() -> String {
        "<Encrypted>".to_string()
    }
}
impl EncryptedState for Encrypted {}
impl ProtectedState for Encrypted {
    fn nonce(&self) -> &[u8; NONCE_SIZE] {
        &self.0
    }

    fn from_nonce(nonce: Vec<u8>) -> Result<Self, SecretError> {
        Ok(Self(nonce.try_into()?))
    }
}
impl CryptoState for Authenticated {
    fn to_string() -> String {
        "<Authenticated>".to_string()
    }
}
impl AuthenticatedState for Authenticated {}
impl ProtectedState for Authenticated {
    fn nonce(&self) -> &[u8; NONCE_SIZE] {
        &self.0
    }

    fn from_nonce(nonce: Vec<u8>) -> Result<Self, SecretError> {
        Ok(Self(nonce.try_into()?, Vec::new()))
    }
}

// ========== Data Container ==========

/// Generic data with state tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data<S: CryptoState> {
    inner: DynamicSecret<Vec<u8>>,
    state: S,
}

impl<S: CryptoState> Data<S> {
    pub fn len(&self) -> usize {
        self.inner.inner_len()
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, SecretError> {
        let mut vec = vec![0; self.len()];
        self.inner.with(|bytes| vec.copy_from_slice(bytes))?;
        Ok(vec)
    }
}

impl<S: CryptoState + ProtectedState> Data<S> {
    pub fn nonce(&self) -> &[u8; NONCE_SIZE] {
        self.state.nonce()
    }

    pub fn as_packet(&self) -> Result<Vec<u8>, SecretError> {
        let len = self.len();
        let mut vec = vec![0; len + NONCE_SIZE];
        vec[len..].copy_from_slice(self.nonce());
        self.inner.with(|bytes| vec[..len].copy_from_slice(bytes))?;
        Ok(vec)
    }

    pub fn from_packet(mut packet: Vec<u8>, tag: impl AsRef<str>) -> Result<Self, SecretError> {
        let state = S::from_nonce(packet.split_off(packet.len() - NONCE_SIZE))?;
        Ok(Self {
            inner: DynamicSecret::new(packet, S::to_string() + tag.as_ref())?,
            state,
        })
    }
}

impl Data<Plain> {
    pub fn new(data: Vec<u8>, tag: impl AsRef<str>) -> Result<Self, SecretError> {
        Ok(Self {
            inner: DynamicSecret::new(data, Plain::to_string() + tag.as_ref())?,
            state: Plain,
        })
    }

    pub fn encrypt<T: NonceTrait>(self, key: &Key<T>) -> Result<Data<Encrypted>, SecretError> {
        let mut nonce = [0u8; NONCE_SIZE];
        let mut dest = DynamicSecret::<Vec<u8>>::new(
            vec![0; self.len() + TAG_SIZE],
            Encrypted::to_string() + self.inner.tag().strip_prefix(&Plain::to_string()).unwrap(),
        )?;

        self.inner.with(|plaintext| {
            dest.with_mut(|dest| key.encrypt(dest, plaintext, &mut nonce, &[]))
        })???;

        Ok(Data {
            inner: dest,
            state: Encrypted(nonce),
        })
    }

    pub fn encrypt_authenticated<T: NonceTrait>(
        self,
        key: &Key<T>,
        associated_data: &[u8],
    ) -> Result<Data<Authenticated>, SecretError> {
        let mut nonce = [0u8; NONCE_SIZE];
        let mut dest = DynamicSecret::<Vec<u8>>::new(
            vec![0; self.len() + TAG_SIZE],
            Authenticated::to_string()
                + self.inner.tag().strip_prefix(&Plain::to_string()).unwrap(),
        )?;

        self.inner.with(|plaintext| {
            dest.with_mut(|dest| key.encrypt(dest, plaintext, &mut nonce, associated_data))
        })???;

        Ok(Data {
            inner: dest,
            state: Authenticated(nonce, associated_data.to_vec()),
        })
    }
}

impl Data<Encrypted> {
    pub fn from_encrypted(inner: DynamicSecret<Vec<u8>>, nonce: [u8; NONCE_SIZE]) -> Self {
        Self {
            inner,
            state: Encrypted(nonce),
        }
    }

    pub fn decrypt<T: NonceTrait>(self, key: &Key<T>) -> Result<Data<Plain>, SecretError> {
        let mut dest = DynamicSecret::<Vec<u8>>::new(
            vec![0; self.len() - TAG_SIZE],
            Plain::to_string()
                + self
                    .inner
                    .tag()
                    .strip_prefix(&Encrypted::to_string())
                    .unwrap(),
        )?;

        self.inner.with(|ciphertext| {
            dest.with_mut(|dest| key.decrypt(dest, ciphertext, &self.state.0, &[]))
        })???;

        Ok(Data {
            inner: dest,
            state: Plain,
        })
    }
}

impl Data<Authenticated> {
    pub fn from_authenticated(
        inner: DynamicSecret<Vec<u8>>,
        nonce: [u8; NONCE_SIZE],
        authenticated_data: Vec<u8>,
    ) -> Self {
        Self {
            inner,
            state: Authenticated(nonce, authenticated_data),
        }
    }

    pub fn authenticated_data(&self) -> &Vec<u8> {
        &self.state.1
    }

    pub fn decrypt_verified<T: NonceTrait>(
        self,
        key: &Key<T>,
        associated_data: &[u8],
    ) -> Result<Data<Plain>, SecretError> {
        let mut dest = DynamicSecret::<Vec<u8>>::new(
            vec![0; self.len() - TAG_SIZE],
            Plain::to_string()
                + self
                    .inner
                    .tag()
                    .strip_prefix(&Authenticated::to_string())
                    .unwrap(),
        )?;

        self.inner.with(|ciphertext| {
            dest.with_mut(|dest| key.decrypt(dest, ciphertext, &self.state.0, associated_data))
        })???;

        Ok(Data {
            inner: dest,
            state: Plain,
        })
    }
}
