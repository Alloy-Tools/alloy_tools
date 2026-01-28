use crate::CryptoError;
use digest::KeyInit;
use hmac::Mac;
use std::marker::PhantomData;
use zeroize::Zeroize;

/// Generic HKDF following RFC 5869.
/// H: Hash function (eg. Blake2s256).
/// K: Key size (eg. 32 for 256-bit keys).
pub struct Hkdf<H: Mac + KeyInit, const N: usize>(PhantomData<H>);

impl<H: Mac + KeyInit, const N: usize> Hkdf<H, N> {
    /// HKDF-Extract (salt, IKM) -> Pseudorandom Key
    pub fn extract(
        dest: &mut [u8; N],
        salt: &[u8],
        initial_key_material: &[u8],
    ) -> Result<(), CryptoError> {
        let mut s = [0u8; N];
        match salt.len() {
            n if n == N => s.copy_from_slice(salt),
            n if n > N => s.copy_from_slice(&salt[..N]),
            n if n < N => s[..n].copy_from_slice(salt),
            _ => {}
        };
        let mut mac = <H as KeyInit>::new_from_slice(&s)?;
        mac.update(initial_key_material);
        let mut result = mac.finalize().into_bytes();
        dest.copy_from_slice(&result[..N]);
        result.zeroize();
        Ok(())
    }

    /// HKDF-Expand (PRK, context, L) -> Output Keying Material
    pub fn expand<const L: usize>(
        dest: &mut [u8; L],
        prk: &[u8; N],
        context: &[u8],
    ) -> Result<(), CryptoError> {
        let n = L.div_ceil(N);
        if n > 255 {
            Err(CryptoError::HkdfExpandTooLong)?
        }

        let mut head = 0;
        let mut t = Vec::new();

        for i in 1..=n {
            let mut mac = <H as KeyInit>::new_from_slice(prk)?;
            mac.update(&t); // T(i-1)
            mac.update(context);
            mac.update(&[i as u8]); // Counter

            t = mac.finalize().into_bytes().to_vec();

            // take min of hash_len (N) or remaining (L - head)
            let taking = N.min(L - head);
            dest[head..head + taking].copy_from_slice(&t[..taking]);
            head += taking;
        }

        Ok(())
    }

    /// Single HKDF (extract + expand)
    pub fn derive<const L: usize>(
        dest: &mut [u8; L],
        salt: &[u8],
        initial_key_material: &[u8],
        context: &[u8],
    ) -> Result<(), CryptoError> {
        let mut prk = [0u8; N];
        Self::extract(&mut prk, salt, initial_key_material)?;
        let result = Self::expand(dest, &prk, context);
        prk.zeroize();
        result
    }

    /// Multiple HKDF.
    /// K is number of keys,
    /// L needs to be `K * KEY_SIZE`.
    pub fn derive_keys<const K: usize, const L: usize>(
        dest: &mut [[u8; N]; K],
        salt: &[u8],
        initial_key_material: &[u8],
        context: &[u8],
    ) -> Result<(), CryptoError> {
        match N.checked_mul(K) {
            Some(total_len) => {
                let mut okm = vec![0u8; total_len];
                Self::derive_slice::<L>(okm.as_mut_slice(), salt, initial_key_material, context)?;
                for i in 0..K {
                    dest[i].copy_from_slice(&okm[i * N..(i + 1) * N]);
                }
                okm.zeroize();
                Ok(())
            }
            None => Err(CryptoError::HkdfExpandTooLong),
        }
    }

    /// Private helper function to call derive with a slice
    fn derive_slice<const L: usize>(
        dest: &mut [u8],
        salt: &[u8],
        initial_key_material: &[u8],
        context: &[u8],
    ) -> Result<(), CryptoError> {
        Self::derive::<L>(
            dest.try_into().map_err(|_| CryptoError::DestTooSmall)?,
            salt,
            initial_key_material,
            context,
        )
    }
}
