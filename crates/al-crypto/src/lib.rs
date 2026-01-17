mod keys;
mod nonces;
mod timestamp;

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
