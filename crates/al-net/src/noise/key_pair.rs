use crate::KEY_SIZE;

pub struct KeyPair;

impl KeyPair {
    //TODO: should generate a new KeyPair
    pub fn new() -> Self {
        Self
    }

    pub fn public(&self) -> &[u8; KEY_SIZE] {
        &[0u8; KEY_SIZE]
    }
}

pub struct PublicKey;

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &[0u8; KEY_SIZE]
    }
}
