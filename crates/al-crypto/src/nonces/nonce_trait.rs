use crate::{
    fill_random, Granularity, Monotonic, MonotonicTimeStamp, Nonce, NonceCounter, NonceError,
    NonceTimestamp, NonceType, RandomTimeStamp,
};

pub trait NonceTrait: super::sealed::Sealed {
    type Graininess: Granularity;
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
}

impl NonceTrait for Monotonic {
    type Graininess = crate::Seconds;

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
