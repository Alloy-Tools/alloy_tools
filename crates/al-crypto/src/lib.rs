mod keys;
mod nonce;

pub use keys::{
    decrypt, derive_pdk, derive_subkey, encrypt, fill_random, from_hex, to_hex, verify_password,
    KEY_SIZE, TAG_SIZE,
};

pub use nonce::{NonceType, NONCE_SIZE};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CryptoError {
    HexError,
    Argon2Error(argon2::Error),
    InvalidKeyLength,
    OsRngError,
    DestToSmall,
    EncryptionError(chacha20poly1305::Error),
    DecryptionError(chacha20poly1305::Error),
}
