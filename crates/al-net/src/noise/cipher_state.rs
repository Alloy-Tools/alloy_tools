use al_crypto::{Monotonic, TAG_SIZE};
use al_vault::{Data, Key, SecretError};
use zeroize::Zeroize;

use crate::{NoiseError, KEY_SIZE};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CipherStateReturn {
    SecretError(SecretError),
    Plaintext(Vec<u8>),
    Ciphertext(Vec<u8>),
}

impl From<SecretError> for CipherStateReturn {
    fn from(value: SecretError) -> Self {
        CipherStateReturn::SecretError(value)
    }
}

pub struct CipherState(Option<Key<Monotonic, KEY_SIZE>>);

impl CipherState {
    pub fn new() -> Self {
        Self(None)
    }

    /// Will take the `key` array into protected memory
    pub fn initialize_key(
        &mut self,
        key: [u8; KEY_SIZE],
        tag: impl Into<String>,
        context: &[u8; 4],
    ) {
        self.0 = Some(Key::from_array(
            key,
            tag,
            al_crypto::Nonce::<Monotonic>::new(context, 0),
        ))
    }

    pub fn has_key(&self) -> bool {
        self.0.is_some()
    }

    pub fn set_nonce(&self, nonce: u64) {
        if let Some(key) = &self.0 {
            let mut key_nonce = match key.nonce().write() {
                Ok(n) => n,
                Err(poisoned) => poisoned.into_inner(),
            };
            key_nonce.set_counter(nonce);
        }
    }

    pub fn encrypt_with_ad(
        &self,
        associated_data: &[u8],
        plaintext: &mut [u8],
    ) -> Result<Vec<u8>, NoiseError> {
        if let Some(key) = &self.0 {
            let data = Data::from_slice(plaintext, "Cipher State Handshake Msg Send")
                .map_err(|e| CipherStateReturn::SecretError(e))?;
            let data = data
                .encrypt_authenticated(key, associated_data)
                .map_err(|e| CipherStateReturn::SecretError(e))?;
            Ok(data.as_packet()?)
        } else {
            Ok(plaintext.to_vec())
        }
    }

    pub fn decrypt_with_ad(
        &self,
        associated_data: &[u8],
        ciphertext_packet: &mut [u8],
    ) -> Result<Vec<u8>, NoiseError> {
        if let Some(key) = &self.0 {
            let data = Data::from_packet(ciphertext_packet, "Cipher State Handshake Msg Recv")
                .map_err(|e| CipherStateReturn::SecretError(e))?;
            let data = data
                .decrypt_verified(key, associated_data)
                .map_err(|e| CipherStateReturn::SecretError(e))?;
            match key.nonce().write() {
                Ok(n) => n,
                Err(poisoned) => poisoned.into_inner(),
            }
            .to_next()
            .map_err(|e| CipherStateReturn::SecretError(SecretError::from(e)))?;
            Ok(data.as_bytes()?)
        } else {
            Ok(ciphertext_packet.to_vec())
        }
    }

    /// Rekeys with Noise specification: k = ENCRYPT(k, maxnonce, zerolen, zeros).
    /// Will return `NonceError:CounterExpired` if `nonce > u64::MAX / 2`, signalling the need for new ephemeral keys.
    pub fn rekey(&mut self) -> Result<(), NoiseError> {
        let new_key = match &self.0 {
            Some(key) => {
                let mut nonce_guard = match key.nonce().write() {
                    Ok(nonce) => nonce,
                    Err(poisoned) => poisoned.into_inner(),
                };

                if let Err(_) = nonce_guard.needs_rotation() {
                    Err(al_crypto::NonceError::CounterExpired)?
                }

                // Save current nonce and set to u64::MAX
                let curr_nonce = nonce_guard.counter_num();
                nonce_guard.set_counter(u64::MAX);

                // Encrypt with max nonce, avoiding nonce checks
                let mut new_key_bytes = [0u8; KEY_SIZE + TAG_SIZE];
                let result = key.encrypt_with(
                    &mut new_key_bytes,
                    &[0u8; KEY_SIZE],
                    nonce_guard.as_bytes(),
                    &[],
                );

                // Set the nonce back and return if encryption was an Err
                nonce_guard.set_counter(curr_nonce);
                result?;

                // Copy the key_bytes and nonce for key creation
                let mut trunc_key = [0u8; KEY_SIZE];
                trunc_key.copy_from_slice(&new_key_bytes[..KEY_SIZE]);
                new_key_bytes.zeroize();
                let mut nonce_bytes = [0u8; al_crypto::NONCE_SIZE];
                nonce_bytes.copy_from_slice(nonce_guard.as_bytes());

                // Return the new key
                Key::<Monotonic, KEY_SIZE>::from_array(
                    trunc_key,
                    key.tag(),
                    al_crypto::Nonce::<Monotonic>::from_bytes(
                        nonce_bytes,
                        nonce_guard.created_at(),
                    ),
                )
            }
            None => return Ok(()),
        };
        self.0 = Some(new_key);
        Ok(())
    }
}
