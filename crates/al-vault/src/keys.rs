use al_crypto::{decrypt, encrypt, CryptoError};

use crate::{Ephemeral, FixedSecret, SecureContainer};

const DEFAULT_KEY_LENGTH: usize = 32;

//TODO:
//pub struct SystemKey<K: KeyType, const N: usize>(SystemSecret<FixedSecret<N, Ephemeral>>, PhantomData<K>);

//TODO system should be able to encrypt Ephemeral secrets by accessing them with SystemControl
/// Keys contain an `Ephemeral` `FixedSecret`
pub struct Key<const N: usize = DEFAULT_KEY_LENGTH>(FixedSecret<N, Ephemeral>);

impl<const N: usize> From<FixedSecret<N, Ephemeral>> for Key<N> {
    fn from(secret: FixedSecret<N, Ephemeral>) -> Self {
        Self(secret)
    }
}

impl<const N: usize> Key<N> {
    pub fn from_array(value: [u8; N], tag: impl Into<String>) -> Self {
        FixedSecret::new(value, tag).into()
    }

    pub fn from_slice(slice: &mut [u8; N], tag: impl Into<String>) -> Self {
        FixedSecret::take(slice, tag).into()
    }

    pub fn random(tag: impl Into<String>) -> Self {
        FixedSecret::<N, Ephemeral>::random(tag).into()
    }

    pub fn encrypt(
        &self,
        dest: &mut [u8],
        plaintext: &[u8],
        nonce: &[u8; al_crypto::NONCE_SIZE],
        associated_data: &[u8],
    ) -> Result<(), CryptoError> {
        encrypt(
            dest,
            plaintext,
            &*self.0.inner.borrow(),
            nonce,
            associated_data,
        )
    }

    pub fn decrypt(
        &self,
        dest: &mut [u8],
        ciphertext: &[u8],
        nonce: &[u8; al_crypto::NONCE_SIZE],
        associated_data: &[u8],
    ) -> Result<(), CryptoError> {
        decrypt(
            dest,
            ciphertext,
            &*self.0.inner.borrow(),
            nonce,
            associated_data,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::keys::Key;
    use al_crypto::{from_hex, to_hex, KEY_SIZE, NONCE_SIZE, TAG_SIZE};
    use secrets::{traits::AsContiguousBytes, SecretVec};

    const TEST_KEY: &str = "4dfc93bf2e50d3f1256fb0550f8d560bee787e80fb4efe3a7c74d9e62ff25755";

    #[test]
    fn encrypt_decrypt() {
        let mut key = [0u8; KEY_SIZE];
        from_hex(TEST_KEY.as_bytes(), &mut key).unwrap();
        let dek: Key<32> = Key::from_array(key, "test dek");

        let mut message: [u8; 14] = b"secret message".as_bytes().try_into().unwrap();
        let secret_msg = SecretVec::<u8>::from(&mut message[..]);
        assert_eq!(message, [0u8; 14]);
        let mut ciphertext = vec![0u8; secret_msg.len() + TAG_SIZE];

        let mut msg_hex = [0u8; 2 * 14];
        {
            assert_eq!(
                str::from_utf8(&secret_msg.borrow()).unwrap(),
                "secret message"
            );
            to_hex(&secret_msg.borrow(), &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "736563726574206d657373616765"
            );
        }
        {
            dek.encrypt(
                ciphertext.as_mut_slice(),
                &secret_msg.borrow(),
                &[1u8; NONCE_SIZE],
                "Key|Test-Enc-Dec".as_bytes(),
            )
            .unwrap();
        }

        let mut hex = [0u8; 2 * (14 + TAG_SIZE)];
        to_hex(&ciphertext, &mut hex).unwrap();
        assert_eq!(
            str::from_utf8(&hex).unwrap(),
            "1d3e27b6249f975eb66f6c04a4635ba982095548576754f9959960876514"
        );

        let decrypted_msg = SecretVec::<u8>::new(ciphertext.len() - al_crypto::TAG_SIZE, |f| {
            dek.decrypt(
                f,
                &ciphertext,
                &[1u8; NONCE_SIZE],
                "Key|Test-Enc-Dec".as_bytes(),
            )
            .unwrap();
        });
        {
            assert_eq!(
                str::from_utf8(&decrypted_msg.borrow()).unwrap(),
                "secret message"
            );
            to_hex(&decrypted_msg.borrow(), &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "736563726574206d657373616765"
            );
        }
    }
}
