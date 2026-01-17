use std::time::{Duration, SystemTime};

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Seconds {}
    impl Sealed for super::Milliseconds {}
    impl Sealed for super::Microseconds {}
}

pub const LIFETIME_THRESHOLD: u32 = 4_000_000_000;

pub trait Granularity: sealed::Sealed {
    const LIFETIME: Duration;

    /// Returns the `Granularity` type
    fn as_granularity() -> TimestampGranularity;

    /// Returns the current timestamp as a byte slice
    fn get_timestamp(epoch: SystemTime) -> [u8; 4];

    /// Returns the current timestamp as a u32
    fn get_timestamp_num(epoch: SystemTime) -> u32 {
        u32::from_be_bytes(Self::get_timestamp(epoch))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Seconds;
impl Granularity for Seconds {
    const LIFETIME: Duration = Duration::from_secs(LIFETIME_THRESHOLD as u64);

    fn as_granularity() -> TimestampGranularity {
        TimestampGranularity::Seconds
    }

    fn get_timestamp(epoch: SystemTime) -> [u8; 4] {
        (get_epoch_timestamp(epoch).as_secs() as u32).to_be_bytes()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Milliseconds;
impl Granularity for Milliseconds {
    const LIFETIME: Duration = Duration::from_millis(LIFETIME_THRESHOLD as u64);

    fn as_granularity() -> TimestampGranularity {
        TimestampGranularity::Milliseconds
    }

    fn get_timestamp(epoch: SystemTime) -> [u8; 4] {
        (get_epoch_timestamp(epoch).as_millis() as u32).to_be_bytes()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Microseconds;
impl Granularity for Microseconds {
    const LIFETIME: Duration = Duration::from_micros(LIFETIME_THRESHOLD as u64);

    fn as_granularity() -> TimestampGranularity {
        TimestampGranularity::Microseconds
    }

    fn get_timestamp(epoch: SystemTime) -> [u8; 4] {
        (get_epoch_timestamp(epoch).as_micros() as u32).to_be_bytes()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum TimestampGranularity {
    #[default]
    Seconds,
    Milliseconds,
    Microseconds,
}

/// Returns the duration since the passed `epoch`
pub fn get_epoch_timestamp(epoch: SystemTime) -> std::time::Duration {
    SystemTime::now().duration_since(epoch).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use crate::{get_epoch_timestamp, Granularity, Microseconds, Milliseconds, Seconds};

    #[test]
    fn epoch_timestamp() {
        assert!(!get_epoch_timestamp(UNIX_EPOCH).is_zero())
    }

    #[test]
    fn seconds() {
        assert!(u32::from_be_bytes(Seconds::get_timestamp(UNIX_EPOCH)) != 0)
    }

    #[test]
    fn milliseconds() {
        assert!(u32::from_be_bytes(Milliseconds::get_timestamp(UNIX_EPOCH)) != 0)
    }

    #[test]
    fn microseconds() {
        assert!(u32::from_be_bytes(Microseconds::get_timestamp(UNIX_EPOCH)) != 0)
    }
}
