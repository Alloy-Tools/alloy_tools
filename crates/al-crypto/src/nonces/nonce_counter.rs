use crate::{Monotonic, MonotonicTimeStamp, NonceTrait, NONCE_SIZE};

pub trait NonceCounter<N: NonceTrait>: super::sealed::Sealed {
    type CounterType;
    /// Returns the max value of the `CounterType`
    fn max() -> Self::CounterType;
    /// Checks if the counter is at or passed the counter threshold
    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool;
    /// Returns the counter potion of the byte slice
    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8];
    /// Returns the counter potion of the byte slice as an unsigned integer
    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType;
    fn set_counter(bytes: &mut [u8; NONCE_SIZE], counter: Self::CounterType);
}

impl NonceCounter<Monotonic> for Monotonic {
    type CounterType = u64;

    fn max() -> Self::CounterType {
        u64::MAX
    }

    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[4..12]
    }

    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType {
        Self::CounterType::from_be_bytes(Self::get_counter(bytes).try_into().unwrap())
    }

    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool {
        Self::get_counter_num(bytes) > Self::CounterType::MAX / 2
    }

    fn set_counter(bytes: &mut [u8; NONCE_SIZE], counter: Self::CounterType) {
        bytes[4..12].copy_from_slice(&counter.to_be_bytes());
    }
}

impl<G: crate::Granularity> NonceCounter<MonotonicTimeStamp<G>> for MonotonicTimeStamp<G> {
    type CounterType = u32;

    fn max() -> Self::CounterType {
        u32::MAX
    }

    fn get_counter(bytes: &[u8; NONCE_SIZE]) -> &[u8] {
        &bytes[8..12]
    }

    fn get_counter_num(bytes: &[u8; NONCE_SIZE]) -> Self::CounterType {
        Self::CounterType::from_be_bytes(Self::get_counter(bytes).try_into().unwrap())
    }

    fn counter_expired(bytes: &[u8; NONCE_SIZE]) -> bool {
        Self::get_counter_num(bytes) > Self::CounterType::MAX / 2
    }

    fn set_counter(bytes: &mut [u8; NONCE_SIZE], counter: Self::CounterType) {
        bytes[8..12].copy_from_slice(&counter.to_be_bytes());
    }
}
