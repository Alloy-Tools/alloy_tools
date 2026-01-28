mod hkdf;
mod keys;
mod nonces;
mod timestamp;

pub use hkdf::Hkdf;
pub use keys::{
    decrypt, derive_pdk, derive_subkey, encrypt, fill_random, from_hex, to_hex, verify_password,
    KEY_SIZE, TAG_SIZE,
};
pub use nonces::{
    nonce::{Nonce, NonceError, NONCE_SIZE},
    nonce_counter::NonceCounter,
    nonce_timestamp::NonceTimestamp,
    nonce_trait::NonceTrait,
    nonce_type::{Monotonic, MonotonicTimeStamp, NonceType, RandomTimeStamp},
};
pub use timestamp::{
    get_epoch_timestamp, Granularity, Microseconds, Milliseconds, Seconds, TimestampGranularity,
    LIFETIME_THRESHOLD,
};

pub type HkdfBlake2s<const N: usize> = Hkdf<hmac::SimpleHmac<blake2::Blake2s256>, N>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CryptoError {
    HexError,
    Argon2Error(argon2::Error),
    InvalidKeyLength,
    OsRngError,
    DestTooSmall,
    HkdfExpandTooLong,
    EncryptionError(chacha20poly1305::Error),
    DecryptionError(chacha20poly1305::Error),
}

pub fn hash<const N: usize>(data: &[u8]) -> [u8; N] {
    use blake2::Digest;
    let mut hasher = blake2::Blake2s256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; N];
    out.copy_from_slice(&result[..N]);
    out
}
