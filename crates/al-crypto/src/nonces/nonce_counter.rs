use crate::{Monotonic, MonotonicTimeStamp, NONCE_SIZE};

pub trait NonceCounter<N: crate::NonceTrait>: super::sealed::Sealed {
    type CounterType;
    /// Checks if the counter is at or passed the counter threshold
    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool;
    /// Returns the counter potion of the byte slice
    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8];
    /// Returns the counter potion of the byte slice as an unsigned integer
    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType;
}

impl NonceCounter<Monotonic> for Monotonic {
    type CounterType = u64;

    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[4..12]
    }

    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType {
        Self::CounterType::from_be_bytes(Self::get_counter(bytes).try_into().unwrap())
    }

    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool {
        Self::get_counter_num(bytes) > Self::CounterType::MAX / 2
    }
}

impl<G: crate::Granularity> NonceCounter<MonotonicTimeStamp<G>> for MonotonicTimeStamp<G> {
    type CounterType = u32;

    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[8..12]
    }

    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType {
        Self::CounterType::from_be_bytes(Self::get_counter(bytes).try_into().unwrap())
    }

    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool {
        Self::get_counter_num(bytes) > Self::CounterType::MAX / 2
    }
}
