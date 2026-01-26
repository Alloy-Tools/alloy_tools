use crate::{Granularity, MonotonicTimeStamp, RandomTimeStamp};
use std::time::SystemTime;

pub trait NonceTimestamp<N: crate::NonceTrait>: super::sealed::Sealed {
    /// Checks if the `Nonce` timestamp portion has expired due to wrapping at maximum age
    fn timestamp_expired(epoch: SystemTime) -> bool {
        SystemTime::now()
            .duration_since(epoch)
            .map(|d| d > N::Graininess::LIFETIME)
            .unwrap_or(true)
    }

    /// Returns the current timestamp as a byte slice
    fn get_timestamp(epoch: SystemTime) -> [u8; 4] {
        N::Graininess::get_timestamp(epoch)
    }

    /// Returns the current timestamp as a u32
    fn get_timestamp_num(epoch: SystemTime) -> u32 {
        N::Graininess::get_timestamp_num(epoch)
    }
}

impl<G: Granularity> NonceTimestamp<MonotonicTimeStamp<G>> for MonotonicTimeStamp<G> {}
impl<G: Granularity> NonceTimestamp<RandomTimeStamp<G>> for RandomTimeStamp<G> {}
