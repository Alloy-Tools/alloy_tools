mod keys;

pub use keys::{
    decrypt, derive_pdk, derive_subkey, encrypt, fill_random, from_hex, to_hex, KEY_SIZE,
    NONCE_SIZE, TAG_SIZE,
};

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
