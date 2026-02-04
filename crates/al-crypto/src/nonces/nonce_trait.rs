use crate::{
    fill_random, Granularity, Monotonic, MonotonicTimeStamp, Nonce, NonceCounter, NonceError,
    NonceTimestamp, NonceType, RandomTimeStamp,
};

pub trait NonceTrait:
    super::sealed::Sealed + Copy + Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static
{
    type Graininess: Granularity;
    /// Returns a new nonce
    fn new(context: &[u8; 4]) -> Result<Nonce<Self>, NonceError>;
    /// Return the `NonceType` of the `Nonce`
    fn nonce_type() -> NonceType;
    /// Returns `Ok(())` if not needed or an `Err(NonceExpiry)` with the reason for expiring
    fn needs_rotation(nonce: &Nonce<Self>) -> Result<(), NonceError>
    where
        Self: Sized;
    /// Cycles the bytes to the next `Nonce`
    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError>
    where
        Self: Sized;
    /// Returns the current counter and sets bytes[4..12] to max. Used by the Noise protocol for rekeying.
    fn set_max(nonce: &mut Nonce<Self>) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&nonce.as_bytes()[4..]);
        nonce.as_bytes_mut()[4..].copy_from_slice(&[255u8; 8]);
        bytes
    }
    /// Sets the nonce counter if using a counter. Used by the Noise protocol for rekeying.
    fn revert_max(nonce: &mut Nonce<Self>, bytes: [u8; 8]) {
        nonce.as_bytes_mut()[4..].copy_from_slice(&bytes)
    }
}

impl NonceTrait for Monotonic {
    type Graininess = crate::Seconds;

    fn new(context: &[u8; 4]) -> Result<Nonce<Self>, NonceError> {
        Ok(Nonce::<Monotonic>::new(context, 0))
    }

    fn nonce_type() -> NonceType {
        NonceType::Monotonic
    }

    fn needs_rotation(nonce: &Nonce<Self>) -> Result<(), NonceError> {
        if Self::counter_expired(nonce.as_bytes()) {
            Err(NonceError::CounterExpired)?;
        }
        Ok(())
    }

    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        Self::needs_rotation(nonce)?;
        let num = u64::from_be_bytes(
            nonce.as_bytes()[4..12]
                .try_into()
                .or_else(|_| Err(NonceError::U64ConvertError))?,
        );
        nonce.as_bytes_mut()[4..12].copy_from_slice(&(num + 1).to_be_bytes());
        Ok(())
    }
}

impl<G: Granularity> NonceTrait for MonotonicTimeStamp<G> {
    type Graininess = G;

    fn new(context: &[u8; 4]) -> Result<Nonce<Self>, NonceError> {
        Ok(Nonce::<MonotonicTimeStamp<G>>::new(context, 0))
    }

    fn nonce_type() -> NonceType {
        NonceType::MonotonicTimeStamp
    }

    fn needs_rotation(nonce: &Nonce<Self>) -> Result<(), NonceError> {
        if Self::timestamp_expired(nonce.get_epoch()) {
            Err(NonceError::TimestampExpired)?;
        }
        if Self::counter_expired(nonce.as_bytes()) {
            Err(NonceError::CounterExpired)?;
        }
        Ok(())
    }

    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        Self::needs_rotation(nonce)?;
        let num = u32::from_be_bytes(
            nonce.as_bytes()[8..12]
                .try_into()
                .or_else(|_| Err(NonceError::U32ConvertError))?,
        );
        nonce.as_bytes_mut()[8..12].copy_from_slice(&(num + 1).to_be_bytes());

        let epoch = nonce.get_epoch();
        nonce.as_bytes_mut()[4..8].copy_from_slice(&G::get_timestamp(epoch));

        Ok(())
    }
}

impl<G: Granularity> NonceTrait for RandomTimeStamp<G> {
    type Graininess = G;

    fn new(context: &[u8; 4]) -> Result<Nonce<Self>, NonceError> {
        Nonce::<RandomTimeStamp<G>>::new(context)
    }

    fn nonce_type() -> NonceType {
        NonceType::RandomTimeStamp
    }

    fn needs_rotation(nonce: &Nonce<Self>) -> Result<(), NonceError> {
        if Self::timestamp_expired(nonce.get_epoch()) {
            Err(NonceError::TimestampExpired)?;
        }
        Ok(())
    }

    fn to_next(nonce: &mut Nonce<Self>) -> Result<(), NonceError> {
        Self::needs_rotation(nonce)?;
        let epoch = nonce.get_epoch();
        nonce.as_bytes_mut()[4..8].copy_from_slice(&G::get_timestamp(epoch));
        Ok(fill_random(&mut nonce.as_bytes_mut()[8..12])
            .map_err(|_| NonceError::FillRandomError)?)
    }
}
