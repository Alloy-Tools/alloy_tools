mod hkdf;
mod keys;
mod nonces;
mod timestamp;

pub use hkdf::Hkdf;
pub use keys::{
    decrypt, derive_pdk, derive_subkey, encrypt, fill_random, from_hex, to_hex, verify_password,
    DHLEN, KEY_SIZE, TAG_SIZE,
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
use zeroize::Zeroize;

pub type HkdfBlake2s<const N: usize> = Hkdf<hmac::SimpleHmac<blake2::Blake2s256>, N>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CryptoError {
    HexError(hex::FromHexError),
    Argon2Error(argon2::Error),
    InvalidKeyLength(digest::InvalidLength),
    OsRngError(rand::rand_core::OsError),
    DestTooSmall(usize, usize),
    HkdfExpandTooLong,
    EncryptionError(chacha20poly1305::aead::Error),
    DecryptionError(chacha20poly1305::aead::Error),
}

// `FromHexError` doesn't impl `Eq` but should.
impl Eq for CryptoError {}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HexError(err) => err.fmt(f),
            Self::Argon2Error(err) => err.fmt(f),
            Self::InvalidKeyLength(err) => err.fmt(f),
            Self::OsRngError(err) => err.fmt(f),
            Self::DestTooSmall(actual, expected) => write!(f, "Destination too small, expected {expected} but found {actual}."),
            Self::HkdfExpandTooLong => write!(f, "HKDF expansion is too long."),
            Self::EncryptionError(err) => err.fmt(f),
            Self::DecryptionError(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CryptoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HexError(err) => err.source(),
            Self::Argon2Error(err) => err.source(),
            Self::InvalidKeyLength(err) => err.source(),
            Self::OsRngError(err) => err.source(),
            _ => None,
        }
    }
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

pub fn diffie_hellman(local_private: [u8; DHLEN], remote_public: [u8; DHLEN]) -> [u8; DHLEN] {
    let mut secret = x25519_dalek::StaticSecret::from(local_private);
    let mut shared = secret.diffie_hellman(&x25519_dalek::PublicKey::from(remote_public));
    let bytes = shared.to_bytes();
    secret.zeroize();
    shared.zeroize();
    bytes
}
