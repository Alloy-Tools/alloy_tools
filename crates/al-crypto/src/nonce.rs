use std::{marker::PhantomData, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::fill_random;

pub const NONCE_SIZE: usize = 12;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Monotonic {}
    impl Sealed for super::MonotonicTimeStamp {}
    impl Sealed for super::RandomTimeStamp {}
}

pub trait NonceTrait: sealed::Sealed {
    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError>
    where
        Self: Sized;

    fn nonce_type() -> NonceType;
}
pub trait NonceTimestamp: sealed::Sealed {}
pub trait NonceCounter: sealed::Sealed {
    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8];
}

impl NonceCounter for Monotonic {
    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[4..12]
    }
}
impl NonceTrait for Monotonic {
    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        let num = u64::from_be_bytes(
            nonce.bytes[4..12]
                .try_into()
                .or_else(|_| Err(NonceError::U64ConvertError))?,
        );
        if num == u64::MAX {
            return Err(NonceError::EndOfCounter);
        }
        nonce.bytes[4..12].copy_from_slice(&(num + 1).to_be_bytes());
        Ok(())
    }

    fn nonce_type() -> NonceType {
        NonceType::Monotonic
    }
}

impl NonceTimestamp for MonotonicTimeStamp {}
impl NonceCounter for MonotonicTimeStamp {
    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[8..12]
    }
}
impl NonceTrait for MonotonicTimeStamp {
    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        let num = u32::from_be_bytes(
            nonce.bytes[8..12]
                .try_into()
                .or_else(|_| Err(NonceError::U32ConvertError))?,
        );
        if num == u32::MAX {
            return Err(NonceError::EndOfCounter);
        }
        nonce.bytes[8..12].copy_from_slice(&(num + 1).to_be_bytes());
        nonce.bytes[4..8].copy_from_slice(&get_timestamp());
        Ok(())
    }

    fn nonce_type() -> NonceType {
        NonceType::MonotonicTimeStamp
    }
}

impl NonceTimestamp for RandomTimeStamp {}
impl NonceTrait for RandomTimeStamp {
    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        nonce.bytes[4..8].copy_from_slice(&get_timestamp());
        fill_random(&mut nonce.bytes[8..12]);
        Ok(())
    }

    fn nonce_type() -> NonceType {
        NonceType::RandomTimeStamp
    }
}

/// Context(4 bytes) || Counter(8 bytes)`~18.4 quintillion` - Simple monotonic counter
pub struct Monotonic;

/// Context(4 bytes) || Timestamp(4 bytes)`~136 years of seconds` || Counter(4 bytes)`~4.2 billion` - Timestamp monotonic counter
pub struct MonotonicTimeStamp;

/// Context(4 bytes) || Timestamp(4 bytes)`~136 years of seconds` || Random(4 bytes)`~4.2 billion` - Timestamp with Random
pub struct RandomTimeStamp;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NonceType {
    Monotonic,
    MonotonicTimeStamp,
    RandomTimeStamp,
}

fn get_timestamp() -> [u8; 4] {
    (SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32)
        .to_be_bytes()
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NonceError {
    U64ConvertError,
    U32ConvertError,
    EndOfCounter,
}

// timestamp = seconds since epoch, truncated to 4 bytes (136 years of seconds)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Nonce<T: NonceTrait> {
    bytes: [u8; NONCE_SIZE],
    _phantom: PhantomData<T>,
}

impl Nonce<Monotonic> {
    pub fn new(context: &[u8; 4], counter: u64) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..12].copy_from_slice(&counter.to_be_bytes());
        Self {
            bytes,
            _phantom: PhantomData,
        }
    }
}

impl Nonce<MonotonicTimeStamp> {
    pub fn new(context: &[u8; 4], counter: u32) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..8].copy_from_slice(&get_timestamp());
        bytes[8..12].copy_from_slice(&counter.to_be_bytes());
        Self {
            bytes,
            _phantom: PhantomData,
        }
    }
}

impl Nonce<RandomTimeStamp> {
    pub fn new(context: &[u8; 4]) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..8].copy_from_slice(&get_timestamp());
        fill_random(&mut bytes[8..]);
        Self {
            bytes,
            _phantom: PhantomData,
        }
    }

    pub fn random(&self) -> &[u8] {
        &self.bytes[8..12]
    }
}

impl<T: NonceTrait + NonceCounter> Nonce<T> {
    pub fn counter(&self) -> &[u8] {
        T::get_counter(&self.bytes)
    }
}

impl<T: NonceTrait + NonceTimestamp> Nonce<T> {
    pub fn timestamp(&self) -> &[u8] {
        &self.bytes[4..8]
    }

    pub fn is_fresh(&self, max_age_seconds: u32) -> bool {
        let curr = u32::from_be_bytes(get_timestamp());
        let stamp = u32::from_be_bytes(self.timestamp().try_into().unwrap_or_default());
        curr.checked_sub(stamp)
            .map(|age| age <= max_age_seconds)
            .unwrap_or(false)
    }
}

impl<T: NonceTrait> Nonce<T> {
    pub fn nonce_type() -> NonceType {
        T::nonce_type()
    }

    pub fn context(&self) -> &[u8] {
        &self.bytes[..4]
    }

    pub fn to_next(&mut self) -> Result<(), NonceError> {
        T::to_next(self)
    }

    pub fn as_bytes(&self) -> &[u8; NONCE_SIZE] {
        &self.bytes
    }

    pub fn to_bytes(&self) -> [u8; NONCE_SIZE] {
        self.bytes.clone()
    }

    pub fn from_bytes(bytes: [u8; NONCE_SIZE]) -> Self {
        Self {
            bytes,
            _phantom: PhantomData,
        }
    }

    pub fn validate(&self, expected_context: &[u8; 4]) -> bool {
        &self.bytes[..4] == expected_context
    }
}
