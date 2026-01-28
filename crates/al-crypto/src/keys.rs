use crate::CryptoError;
use chacha20poly1305::{aead::AeadMutInPlace, ChaCha20Poly1305};
use rand::{rngs::OsRng, TryRngCore};
use zeroize::Zeroize;

pub const TAG_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;

impl From<argon2::Error> for CryptoError {
    fn from(error: argon2::Error) -> Self {
        CryptoError::Argon2Error(error)
    }
}

impl From<digest::InvalidLength> for CryptoError {
    fn from(_: digest::InvalidLength) -> Self {
        CryptoError::InvalidKeyLength
    }
}

impl From<rand::rand_core::OsError> for CryptoError {
    fn from(_: rand::rand_core::OsError) -> Self {
        CryptoError::OsRngError
    }
}

impl From<hex::FromHexError> for CryptoError {
    fn from(_: hex::FromHexError) -> Self {
        CryptoError::HexError
    }
}

/// Doesn't zeroize the passed `bytes`
pub fn to_hex(bytes: &[u8], str: &mut [u8]) -> Result<(), CryptoError> {
    Ok(hex::encode_to_slice(bytes, str)?)
}

/// Doesn't zeroize the passed `str`
pub fn from_hex(str: &[u8], bytes: &mut [u8]) -> Result<(), CryptoError> {
    Ok(hex::decode_to_slice(str, bytes)?)
}

/// Fills the `dest` byte array with random bytes
pub fn fill_random(dest: &mut [u8]) -> Result<(), CryptoError> {
    Ok(OsRng.try_fill_bytes(dest)?)
}
/// Doesn't zeroize the passed `password`, `salt`, or `expected_pdk`
pub fn verify_password<const N: usize>(
    password: &[u8],
    salt: &[u8],
    expected_pdk: &[u8; N],
) -> Result<bool, CryptoError> {
    let mut pdk = [0u8; N];
    derive_pdk(&mut pdk, password, salt)?;
    let result = &pdk == expected_pdk;
    pdk.zeroize();
    Ok(result)
}

/// Doesn't zeroize the passed `password` or `salt`
pub fn derive_pdk<const N: usize>(
    dest: &mut [u8; N],
    password: &[u8],
    salt: &[u8],
) -> Result<(), CryptoError> {
    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(65536, 3, 1, Some(N))?,
    );
    Ok(argon2.hash_password_into(password, &salt, dest)?)
}

/// Doesn't zeroize the passed `key` or `context`
pub fn derive_subkey<const N: usize>(
    dest: &mut [u8; N],
    key: &[u8],
    context: impl AsRef<str>,
) -> Result<(), CryptoError> {
    crate::HkdfBlake2s::<N>::derive(dest, &[], key, context.as_ref().as_bytes())
}

/// Encrypts the `plaintext` byte slice into the `dest` byte slice using the `key`, `nonce`, and `associated_data` slices
pub fn encrypt(
    dest: &mut [u8],
    plaintext: &[u8],
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
) -> Result<(), CryptoError> {
    let p_len = plaintext.len();
    if dest.len() < p_len + TAG_SIZE {
        return Err(CryptoError::DestTooSmall);
    }
    let mut cipher = <ChaCha20Poly1305 as digest::KeyInit>::new_from_slice(key)?;
    dest[..p_len].copy_from_slice(plaintext);
    let tag = cipher
        .encrypt_in_place_detached(nonce.into(), associated_data, &mut dest[..p_len])
        .map_err(|e| {
            dest.zeroize();
            CryptoError::EncryptionError(e)
        })?;
    dest[p_len..p_len + TAG_SIZE].copy_from_slice(&tag);
    Ok(())
}

/// Decrypts the `ciphertext` byte slice into the `dest` byte slice using the `key`, `nonce`, and `associated_data` slices
pub fn decrypt(
    dest: &mut [u8],
    ciphertext: &[u8],
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
) -> Result<(), CryptoError> {
    let p_len = ciphertext.len() - TAG_SIZE;
    if dest.len() < p_len {
        return Err(CryptoError::DestTooSmall);
    }
    let mut cipher = <ChaCha20Poly1305 as digest::KeyInit>::new_from_slice(key)?;

    let (data, tag) = ciphertext.split_at(p_len);
    dest.copy_from_slice(data);

    cipher
        .decrypt_in_place_detached(nonce.into(), associated_data, dest, tag.into())
        .map_err(|e| {
            dest.zeroize();
            CryptoError::DecryptionError(e)
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroize;

    use crate::{decrypt, derive_pdk, derive_subkey, encrypt, to_hex, KEY_SIZE, NONCE_SIZE};

    const TEST_PASSWORD: &str = "SecretPassword1";
    const TEST_SALT: &str = "1234567890123456";
    const TEST_PASSWORD2: &str = "SecretPassword2";
    const TEST_SALT2: &str = "6234567890123451";
    const SUBKEY_CONTEXT: &str = "subkey-testing";

    #[test]
    fn pdk() {
        let mut pdk = [0u8; KEY_SIZE];
        let mut hex = [0u8; 2 * KEY_SIZE];
        derive_pdk(&mut pdk, TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes()).unwrap();
        to_hex(&pdk, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "05546d4b0a40d2c5b686ac39ca3d104a4d7e1b7ee0352b0b470aaad1943e7fd0"
        )
    }

    #[test]
    fn verify_password() {
        let mut pdk = [0u8; KEY_SIZE];
        derive_pdk(&mut pdk, TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes()).unwrap();
        assert_eq!(
            crate::verify_password(TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes(), &pdk).unwrap(),
            true
        );
        assert_eq!(
            crate::verify_password(TEST_PASSWORD2.as_bytes(), TEST_SALT.as_bytes(), &pdk).unwrap(),
            false
        );
        assert_eq!(
            crate::verify_password(TEST_PASSWORD.as_bytes(), TEST_SALT2.as_bytes(), &pdk).unwrap(),
            false
        );
    }

    #[test]
    fn full_pdk() {
        let mut dest = [0u8; KEY_SIZE];
        let mut hex = [0u8; 2 * KEY_SIZE];
        derive_pdk(&mut dest, TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes()).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "05546d4b0a40d2c5b686ac39ca3d104a4d7e1b7ee0352b0b470aaad1943e7fd0"
        );
        derive_pdk(&mut dest, TEST_PASSWORD2.as_bytes(), TEST_SALT.as_bytes()).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "403ba84bea63d0771b86636f93f4eb2c1aca113595250ae71e3da4302a5a5ea3"
        );
        derive_pdk(&mut dest, TEST_PASSWORD.as_bytes(), TEST_SALT2.as_bytes()).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "d9dd64e55fd6f5eef122e5d7eeace6155125b58510c7042ff0f993f3a6cb5f4e"
        );
        derive_pdk(&mut dest, TEST_PASSWORD2.as_bytes(), TEST_SALT2.as_bytes()).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "24f28f1f6fe288ef6cc54f10f6cec384ecd6272008783ea08e88e2e6d309c509"
        );
    }

    #[test]
    fn subkey() {
        let mut key = [0u8; KEY_SIZE];
        derive_pdk(&mut key, TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes()).unwrap();

        let mut dest = [0u8; KEY_SIZE];
        let mut hex = [0u8; 2 * KEY_SIZE];
        derive_subkey(&mut dest, &key, SUBKEY_CONTEXT).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "ee3c7f556497600463e1744e0300460cd91f9c2fec3eb664da59ac50e855c130"
        );
        let mut new_dest = [0u8; KEY_SIZE];
        derive_subkey(&mut new_dest, &dest, SUBKEY_CONTEXT).unwrap();
        to_hex(&new_dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "56e489124201e4f6bfb9b4e12bf1eaed032367bb86f157afb897821d42a39945"
        )
    }

    #[test]
    fn encrypt_decrypt() {
        let mut key = [0u8; 32];
        derive_pdk(&mut key, TEST_PASSWORD.as_bytes(), TEST_SALT.as_bytes()).unwrap();

        let mut message = [0u8; 14];
        message.copy_from_slice("secret message".as_bytes());

        let mut ciphertext = [0u8; 30]; // message.len() + 16
        let mut hex = [0u8; 60]; // ciphertext.len() * 2
        encrypt(
            &mut ciphertext,
            &mut message,
            &key,
            &[1u8; NONCE_SIZE],
            "Enc|Test".as_bytes(),
        )
        .unwrap();
        message.zeroize();
        to_hex(&ciphertext, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "4ab4e7efa03ffd7474e866f770198606146e04c7da0e05905c1d6fce2b9d"
        );
        decrypt(
            &mut message,
            &ciphertext,
            &key,
            &[1u8; NONCE_SIZE],
            "Enc|Test".as_bytes(),
        )
        .unwrap();
        assert_eq!(str::from_utf8(&message).unwrap(), "secret message");
    }
}
