use crate::{
    fill_random, timestamp::Granularity, Monotonic, MonotonicTimeStamp, NonceCounter,
    NonceTimestamp, NonceTrait, NonceType, RandomTimeStamp,
};
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const NONCE_SIZE: usize = 12;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NonceError {
    U64ConvertError,
    U32ConvertError,
    CounterExpired,
    TimestampExpired,
    FillRandomError,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Nonce<T: NonceTrait> {
    bytes: [u8; NONCE_SIZE],
    created_at: u64,
    _phantom: PhantomData<T>,
}

impl Nonce<Monotonic> {
    // Creates a new `Nonce` of type `Monotonic`
    pub fn new(context: &[u8; 4], counter: u64) -> Self {
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..12].copy_from_slice(&counter.to_be_bytes());
        Self {
            bytes,
            created_at: Self::created_now(),
            _phantom: PhantomData,
        }
    }
}

impl<G: Granularity> Nonce<MonotonicTimeStamp<G>> {
    // Creates a new `Nonce` of type `MonotonicTimeStamp`
    pub fn new(context: &[u8; 4], counter: u32) -> Self {
        let created_at = Self::created_now();
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..8].copy_from_slice(&G::get_timestamp(Self::epoch_from(created_at)));
        bytes[8..12].copy_from_slice(&counter.to_be_bytes());
        Self {
            bytes,
            created_at,
            _phantom: PhantomData,
        }
    }
}

impl<G: Granularity> Nonce<RandomTimeStamp<G>> {
    /// Returns a new `Ok(Nonce)` of type `RandomTimeStamp`, or `Err` if the random byte generation fails
    pub fn new(context: &[u8; 4]) -> Result<Self, NonceError> {
        let created_at = Self::created_now();
        let mut bytes = [0u8; NONCE_SIZE];
        bytes[..4].copy_from_slice(context);
        bytes[4..8].copy_from_slice(&G::get_timestamp(Self::epoch_from(created_at)));
        fill_random(&mut bytes[8..]).map_err(|_| NonceError::FillRandomError)?;
        Ok(Self {
            bytes,
            created_at,
            _phantom: PhantomData,
        })
    }

    /// Retuns the `Nonce` random byte portion
    pub fn random(&self) -> &[u8] {
        &self.bytes[8..12]
    }

    /// Retuns the `Nonce` random byte portion as u32
    pub fn random_num(&self) -> u32 {
        u32::from_be_bytes(Self::random(&self).try_into().unwrap())
    }
}

impl<T: NonceTrait + NonceCounter<T>> Nonce<T> {
    /// Retuns the `Nonce` counter byte portion
    pub fn counter(&self) -> &[u8] {
        T::get_counter(&self.bytes)
    }

    /// Retuns the `Nonce` counter byte portion as an unsigned integer
    pub fn counter_num(&self) -> T::CounterType {
        T::get_counter_num(&self.bytes)
    }
}

impl<T: NonceTrait + NonceTimestamp<T>> Nonce<T> {
    /// Returns the `Nonce` timestamp byte portion
    pub fn timestamp(&self) -> &[u8] {
        &self.bytes[4..8]
    }

    /// Returns the `Nonce` timestamp byte portion as a u32
    pub fn timestamp_num(&self) -> u32 {
        T::get_timestamp_num(self.get_epoch())
    }

    /// Checks if the `Nonce` timestamp is less than `max_age`
    pub fn is_fresh(&self, max_age: u32) -> bool {
        let curr = u32::from_be_bytes(T::get_timestamp(self.get_epoch()));
        let stamp = u32::from_be_bytes(self.timestamp().try_into().unwrap_or_default());
        curr.checked_sub(stamp)
            .map(|age| age <= max_age)
            .unwrap_or(false)
    }
}

impl<T: NonceTrait> Nonce<T> {
    /// Returns the current microseconds since the UNIX_EPOCH as u64, covering ~584 thousand years.
    fn created_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    /// Returns the epoch as the sum of the UNIX EPOXH and the passed microseconds
    fn epoch_from(micros: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_micros(micros)
    }

    /// Returns the epoch as the sum of the UNIX EPOXH and the creation time
    pub fn get_epoch(&self) -> SystemTime {
        Self::epoch_from(self.created_at)
    }

    /// Return the `NonceType` of the `Nonce`
    pub fn nonce_type(&self) -> NonceType {
        T::nonce_type()
    }

    /// Returns the context portion of bytes
    pub fn context(&self) -> &[u8] {
        &self.bytes[..4]
    }

    /// Cycles the bytes to the next `Nonce`
    pub fn to_next(&mut self) -> Result<(), NonceError> {
        T::to_next(self)
    }

    /// Returns a reference to the `Nonce` bytes
    pub fn as_bytes(&self) -> &[u8; NONCE_SIZE] {
        &self.bytes
    }

    /// Returns a mutable reference to the `Nonce` bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8; NONCE_SIZE] {
        &mut self.bytes
    }

    /// Returns a clone of the `Nonce` bytes
    pub fn to_bytes(&self) -> [u8; NONCE_SIZE] {
        self.bytes.clone()
    }

    /// Creates a `Nonce` from bytes and the epoch time
    pub fn from_bytes(bytes: [u8; NONCE_SIZE], nonce_epoch: u64) -> Self {
        Self {
            bytes,
            created_at: nonce_epoch,
            _phantom: PhantomData,
        }
    }

    /// Validates the context portion of the `Nonce` bytes
    pub fn validate_context(&self, expected_context: &[u8; 4]) -> bool {
        &self.bytes[..4] == expected_context
    }

    /// Returns `Ok(())` if not needed or an `Err(NonceExpiry)` with the reason for expiring
    pub fn needs_rotation(&self) -> Result<(), NonceError> {
        T::needs_rotation(self)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread::sleep,
        time::{Duration, SystemTime, UNIX_EPOCH},
        u64,
    };

    use crate::{
        fill_random, to_hex, Microseconds, Milliseconds, Monotonic, MonotonicTimeStamp, Nonce,
        NonceError, NonceType, RandomTimeStamp, Seconds, NONCE_SIZE,
    };

    const TEST_CONTEXT: &[u8; 4] = b"TEST";

    #[test]
    fn byte_conversion() {
        let nonce = Nonce::<Monotonic>::new(TEST_CONTEXT, 42);
        let bytes = nonce.to_bytes();
        let new_nonce = Nonce::<Monotonic>::from_bytes(bytes, nonce.created_at);
        assert!(new_nonce.validate_context(TEST_CONTEXT));
        assert_eq!(new_nonce.counter_num(), 42);

        let nonce = Nonce::<MonotonicTimeStamp<Microseconds>>::new(TEST_CONTEXT, 42);
        let bytes = nonce.to_bytes();
        let new_nonce =
            Nonce::<MonotonicTimeStamp<Microseconds>>::from_bytes(bytes, nonce.created_at);
        assert!(new_nonce.validate_context(TEST_CONTEXT));
        assert_eq!(new_nonce.counter_num(), 42);

        let nonce = Nonce::<RandomTimeStamp<Microseconds>>::new(TEST_CONTEXT).unwrap();
        let bytes = nonce.to_bytes();
        let new_nonce = Nonce::<RandomTimeStamp<Microseconds>>::from_bytes(bytes, nonce.created_at);
        assert!(new_nonce.validate_context(TEST_CONTEXT));
        assert_eq!(new_nonce.random(), &bytes[8..12]);
    }

    #[test]
    fn rotation_checks() {
        // Monotonic
        let mut nonce = Nonce::<Monotonic>::new(TEST_CONTEXT, 0);
        assert_eq!(nonce.to_next().unwrap(), ());

        let mut nonce = Nonce::<Monotonic>::new(TEST_CONTEXT, u64::MAX);
        assert_eq!(nonce.to_next().unwrap_err(), NonceError::CounterExpired);

        // MonotonicTimeStamp
        let mut nonce = Nonce::<MonotonicTimeStamp<Microseconds>>::new(TEST_CONTEXT, 0);
        assert_eq!(nonce.to_next().unwrap(), ());

        let mut nonce = Nonce::<MonotonicTimeStamp<Microseconds>>::new(TEST_CONTEXT, u32::MAX);
        assert_eq!(nonce.to_next().unwrap_err(), NonceError::CounterExpired);

        let mut nonce = Nonce::<MonotonicTimeStamp<Microseconds>>::from_bytes(
            [0u8; NONCE_SIZE],
            (SystemTime::now()
                .checked_sub(Duration::from_secs(4200)) // 70 minutes
                .unwrap())
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
        );
        assert_eq!(nonce.to_next().unwrap_err(), NonceError::TimestampExpired);

        // RandomTimeStamp
        let mut nonce = Nonce::<RandomTimeStamp<Microseconds>>::new(TEST_CONTEXT).unwrap();
        assert_eq!(nonce.to_next().unwrap(), ());

        let mut nonce = Nonce::<RandomTimeStamp<Microseconds>>::from_bytes(
            [0u8; NONCE_SIZE],
            (SystemTime::now()
                .checked_sub(Duration::from_secs(4200)) // 70 minutes
                .unwrap())
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
        );
        assert_eq!(nonce.to_next().unwrap_err(), NonceError::TimestampExpired);
    }

    #[test]
    fn monotonic() {
        let mut nonce = Nonce::<Monotonic>::new(TEST_CONTEXT, 0);
        let mut hex = [0u8; NONCE_SIZE * 2];

        to_hex(nonce.as_bytes(), &mut hex).unwrap();
        assert_eq!("544553540000000000000000", str::from_utf8(&hex).unwrap());
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::Monotonic);
        assert_eq!(nonce.counter_num(), 0);

        nonce.to_next().unwrap();

        to_hex(nonce.as_bytes(), &mut hex).unwrap();
        assert_eq!("544553540000000000000001", str::from_utf8(&hex).unwrap());
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::Monotonic);
        assert_eq!(nonce.counter_num(), 1);
    }

    #[test]
    fn monotonic_timestamp() {
        let mut nonce = Nonce::<MonotonicTimeStamp<Seconds>>::new(TEST_CONTEXT, 0);
        sleep(Duration::from_secs(1));

        let timestamp = nonce.timestamp_num();
        assert_ne!(timestamp, 0);
        assert_eq!(nonce.counter_num(), 0);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_secs(1));

        let timestamp2 = nonce.timestamp_num();
        assert_ne!(timestamp2, 0);
        assert_eq!(nonce.counter_num(), 1);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);
        assert!(timestamp < timestamp2);

        let mut nonce = Nonce::<MonotonicTimeStamp<Milliseconds>>::new(TEST_CONTEXT, 0);
        sleep(Duration::from_millis(1));

        let timestamp = nonce.timestamp_num();
        assert_ne!(timestamp, 0);
        assert_eq!(nonce.counter_num(), 0);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_millis(1));

        let timestamp2 = nonce.timestamp_num();
        assert_ne!(timestamp2, 0);
        assert_eq!(nonce.counter_num(), 1);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);
        assert!(timestamp < timestamp2);

        let mut nonce = Nonce::<MonotonicTimeStamp<Microseconds>>::new(TEST_CONTEXT, 0);
        sleep(Duration::from_micros(1));

        let timestamp = nonce.timestamp_num();
        assert_ne!(timestamp, 0);
        assert_eq!(nonce.counter_num(), 0);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_micros(1));

        let timestamp2 = nonce.timestamp_num();
        assert_ne!(timestamp2, 0);
        assert_eq!(nonce.counter_num(), 1);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::MonotonicTimeStamp);
        assert!(timestamp < timestamp2);
    }

    #[test]
    fn random_timestamp() {
        let test_rand = {
            let mut random_bytes = [0u8; 4];
            fill_random(&mut random_bytes).unwrap();
            u32::from_be_bytes(random_bytes)
        };

        let mut nonce = Nonce::<RandomTimeStamp<Seconds>>::new(TEST_CONTEXT).unwrap();
        sleep(Duration::from_secs(1));

        let timestamp = nonce.timestamp_num();
        let random = nonce.random_num();
        assert_ne!(timestamp, 0);
        assert_ne!(random, test_rand);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_secs(1));

        let timestamp2 = nonce.timestamp_num();
        let random2 = nonce.random_num();
        assert!(timestamp < timestamp2);
        assert_ne!(timestamp2, 0);
        assert_ne!(random2, test_rand);
        assert_ne!(random2, random);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);

        let mut nonce = Nonce::<RandomTimeStamp<Milliseconds>>::new(TEST_CONTEXT).unwrap();
        sleep(Duration::from_millis(1));

        let timestamp = nonce.timestamp_num();
        let random = nonce.random_num();
        assert_ne!(timestamp, 0);
        assert_ne!(random, test_rand);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_millis(1));

        let timestamp2 = nonce.timestamp_num();
        let random2 = nonce.random_num();
        assert!(timestamp < timestamp2);
        assert_ne!(timestamp2, 0);
        assert_ne!(random2, test_rand);
        assert_ne!(random2, random);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);

        let mut nonce = Nonce::<RandomTimeStamp<Microseconds>>::new(TEST_CONTEXT).unwrap();
        sleep(Duration::from_micros(1));

        let timestamp = nonce.timestamp_num();
        let random = nonce.random_num();
        assert_ne!(timestamp, 0);
        assert_ne!(random, test_rand);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);

        nonce.to_next().unwrap();
        sleep(Duration::from_micros(1));

        let timestamp2 = nonce.timestamp_num();
        let random2 = nonce.random_num();
        assert!(timestamp < timestamp2);
        assert_ne!(timestamp2, 0);
        assert_ne!(random2, test_rand);
        assert_ne!(random2, random);
        assert!(nonce.validate_context(TEST_CONTEXT));
        assert_eq!(nonce.context(), TEST_CONTEXT);
        assert_eq!(nonce.nonce_type(), NonceType::RandomTimeStamp);
    }
}
