use crate::CryptoError;
use argon2::Argon2;
use blake2::Blake2s256;
use chacha20poly1305::{aead::AeadMutInPlace, ChaCha20Poly1305};
use hmac::{Mac, SimpleHmac};
use rand::{rngs::OsRng, TryRngCore};
use zeroize::Zeroize;

pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;

impl From<argon2::Error> for CryptoError {
    fn from(error: argon2::Error) -> Self {
        CryptoError::Argon2Error(error)
    }
}

impl From<blake2::digest::InvalidLength> for CryptoError {
    fn from(_: blake2::digest::InvalidLength) -> Self {
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

pub fn to_hex(bytes: &[u8], str: &mut [u8]) -> Result<(), CryptoError> {
    Ok(hex::encode_to_slice(bytes, str)?)
}

pub fn from_hex(str: &[u8], bytes: &mut [u8]) -> Result<(), CryptoError> {
    Ok(hex::decode_to_slice(str, bytes)?)
}

pub fn fill_random(dest: &mut [u8]) -> Result<(), CryptoError> {
    Ok(OsRng.try_fill_bytes(dest)?)
}

pub fn derive_pdk<const N: usize>(
    dest: &mut [u8; N],
    password: &[u8],
    salt: &[u8],
) -> Result<(), CryptoError> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(65536, 3, 1, Some(N))?,
    );
    Ok(argon2.hash_password_into(password, &salt, dest)?)
}

pub fn derive_subkey<const N: usize>(
    dest: &mut [u8; N],
    key: &[u8],
    context: impl AsRef<str>,
) -> Result<(), CryptoError> {
    let mut mac = SimpleHmac::<Blake2s256>::new_from_slice(key)?;
    mac.update(context.as_ref().as_bytes());
    let mut result = mac.finalize().into_bytes();
    dest.copy_from_slice(&result[..N]);
    zeroize::Zeroize::zeroize(&mut result[..]);
    Ok(())
}

pub fn encrypt(
    dest: &mut [u8],
    plaintext: &[u8],
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
) -> Result<(), CryptoError> {
    let p_len = plaintext.len();
    if dest.len() < p_len + TAG_SIZE {
        return Err(CryptoError::DestToSmall);
    }
    let mut cipher = <ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new_from_slice(key)?;
    dest[..p_len].copy_from_slice(plaintext);
    let tag = cipher
        .encrypt_in_place_detached(nonce.into(), associated_data, &mut dest[..p_len])
        .map_err(|e| {
            dest.zeroize();
            CryptoError::EncryptionError(e)
        })?;
    dest[p_len..].copy_from_slice(&tag);
    Ok(())
}

pub fn decrypt(
    dest: &mut [u8],
    ciphertext: &[u8],
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
) -> Result<(), CryptoError> {
    if dest.len() < ciphertext.len() - TAG_SIZE {
        return Err(CryptoError::DestToSmall);
    }
    let mut cipher = <ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new_from_slice(key)?;

    let (data, tag) = ciphertext.split_at(ciphertext.len() - TAG_SIZE);
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

    use crate::{
        keys::{decrypt, derive_pdk, derive_subkey, encrypt, to_hex, NONCE_SIZE},
        KEY_SIZE,
    };

    const TEST_PASSWORD: &str = "SecretPassword1";
    const TEST_SALT: &str = "1234567890123456";
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
    fn full_pdk() {
        let mut dest = [0u8; KEY_SIZE];
        let mut hex = [0u8; 2 * KEY_SIZE];
        let password = TEST_PASSWORD.as_bytes();
        let salt = TEST_SALT.as_bytes();
        let password2 = "SecretPassword2".as_bytes();
        let salt2 = "6234567890123451".as_bytes();
        derive_pdk(&mut dest, password, salt).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "05546d4b0a40d2c5b686ac39ca3d104a4d7e1b7ee0352b0b470aaad1943e7fd0"
        );
        derive_pdk(&mut dest, password2, salt).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "403ba84bea63d0771b86636f93f4eb2c1aca113595250ae71e3da4302a5a5ea3"
        );
        derive_pdk(&mut dest, password, salt2).unwrap();
        to_hex(&dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "d9dd64e55fd6f5eef122e5d7eeace6155125b58510c7042ff0f993f3a6cb5f4e"
        );
        derive_pdk(&mut dest, password2, salt2).unwrap();
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
            "c9dae20c4c81978621940ca6663adcd90da1678e2185afd2e9e0e0900d0b3e89"
        );
        let mut new_dest = [0u8; KEY_SIZE];
        derive_subkey(&mut new_dest, &dest, SUBKEY_CONTEXT).unwrap();
        to_hex(&new_dest, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "ab2f2446b748406b6daaf7cb2f8b4123de1824a309d7487e37f192ca285401e3"
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
