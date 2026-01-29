use crate::{
    CipherState, HandshakePattern, NoiseError, DOUBLE_KEY_SIZE, HASHLEN, KEY_SIZE, TRIPLE_KEY_SIZE,
};
use al_crypto::{hash, HkdfBlake2s};
use al_vault::{FixedSecret, SecureAccess};
use zeroize::Zeroize;

pub type SplitResult = (CipherState, CipherState, [u8; HASHLEN]);

pub struct SymmetricState {
    cipher_state: CipherState,
    ck: FixedSecret<HASHLEN>,
    h: FixedSecret<HASHLEN>,
}

impl SymmetricState {
    pub fn initialize_symmetric(pattern: HandshakePattern) -> Self {
        Self {
            cipher_state: CipherState::new(),
            ck: FixedSecret::new(pattern.to_bytes(), "ck"),
            h: FixedSecret::new(pattern.to_bytes(), "h"),
        }
    }

    pub fn has_key(&self) -> bool {
        self.cipher_state.has_key()
    }

    pub fn mix_key(&mut self, input_key_material: &[u8]) -> Result<(), NoiseError> {
        self.ck.with_mut(|ck| {
            let mut keys = [[0u8; KEY_SIZE]; 2];
            HkdfBlake2s::derive_keys::<2, DOUBLE_KEY_SIZE>(&mut keys, ck, input_key_material, &[])?;
            ck.copy_from_slice(&keys[0]);
            self.cipher_state
                .initialize_key(keys[1], "Cipherstate Key", b"CKEY");
            keys.zeroize();
            Ok(())
        })
    }

    pub fn mix_hash(&mut self, data: &[u8]) {
        self.h.with_mut(|h| {
            let temp = h.clone();
            h.copy_from_slice(&hash::<HASHLEN>(&[&temp, data].concat()))
        })
    }

    pub fn mix_key_and_hash(&mut self, input_key_material: &[u8]) -> Result<(), NoiseError> {
        let mut temp_h = [0u8; KEY_SIZE];
        self.ck.with_mut(|ck| {
            let mut keys = [[0u8; KEY_SIZE]; 3];
            HkdfBlake2s::derive_keys::<3, TRIPLE_KEY_SIZE>(&mut keys, ck, input_key_material, &[])?;
            ck.copy_from_slice(&keys[0]);
            temp_h.copy_from_slice(&keys[1]);
            self.cipher_state
                .initialize_key(keys[2], "Cipherstate Key", b"CKEY");
            keys.zeroize();
            Ok::<(), NoiseError>(())
        })?;
        self.mix_hash(&temp_h);
        temp_h.zeroize();
        Ok(())
    }

    pub fn get_handshake_hash(&self) -> [u8; HASHLEN] {
        self.h.copy()
    }

    pub fn encrypt_and_hash(&mut self, plaintext: &mut [u8]) -> Result<Vec<u8>, NoiseError> {
        let ciphertext = self
            .h
            .with(|h| self.cipher_state.encrypt_with_ad(h, plaintext))?;
        self.mix_hash(&ciphertext);
        Ok(ciphertext)
    }

    pub fn decrypt_and_hash(
        &mut self,
        ciphertext_packet: &mut [u8],
    ) -> Result<Vec<u8>, NoiseError> {
        let packet = ciphertext_packet.to_vec();
        let plaintext = self
            .h
            .with(|h| self.cipher_state.decrypt_with_ad(h, ciphertext_packet))?;
        self.mix_hash(&packet);
        Ok(plaintext)
    }

    pub fn split(&self) -> Result<SplitResult, NoiseError> {
        self.ck.with(|ck| {
            let mut keys = [[0u8; KEY_SIZE]; 2];
            HkdfBlake2s::derive_keys::<2, DOUBLE_KEY_SIZE>(&mut keys, ck, &[], &[])?;
            let mut c1 = CipherState::new();
            let mut c2 = CipherState::new();

            let mut key = [0u8; KEY_SIZE];
            key.copy_from_slice(&keys[0]);
            c1.initialize_key(key, "Noise Send Key", b"SKEY");
            key.copy_from_slice(&keys[1]);
            c2.initialize_key(key, "Noise Recv Key", b"RKEY");
            keys.zeroize();

            Ok((c1, c2, self.get_handshake_hash()))
        })
    }
}
