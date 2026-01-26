mod async_executor;
mod audit;
mod container;
mod secrets;

#[cfg(feature = "tokio")]
pub use async_executor::{TokioExecutor, TokioJoinHandle};
pub use audit::{AuditEntry, AuditError, AuditLog, AUDIT_LOG, AUDIT_LOG_CAPACITY};
pub use container::{
    secure_container::{SecureAccess, SecureContainer},
    security_level::{AsSecurityLevel, Ephemeral, Persistent, SecurityLevel},
};
pub use secrets::{
    data::{
        Authenticated, AuthenticatedState, CryptoState, Data, Encrypted, EncryptedState, Plain,
        PlainState,
    },
    dynamic_secret::DynamicSecret,
    fixed_secret::FixedSecret,
    keys::Key,
    secret_error::SecretError,
    secure_ref::{SecureRef, Secureable},
};

#[cfg(test)]
mod tests {
    use crate::{Data, Key};
    use al_crypto::{from_hex, to_hex, Monotonic, Nonce, KEY_SIZE};

    const TEST_KEY: &str = "4dfc93bf2e50d3f1256fb0550f8d560bee787e80fb4efe3a7c74d9e62ff25755";
    const TEST_ASSOCIATED_DATA: &[u8; 16] = b"Key|Test-Enc-Dec";

    #[tokio::test]
    async fn encrypt_decrypt() {
        // init the key
        let dek = {
            let mut key = [0u8; KEY_SIZE];
            from_hex(TEST_KEY.as_bytes(), &mut key).unwrap();
            Key::from_array(key, "Test DEK", Nonce::<Monotonic>::new(b"Test", 0))
        };

        // gather user input
        let mut user_input = String::new();
        user_input.push_str("secret message");
        let plaintext = Data::new(user_input.into_bytes(), "Local Secret").unwrap();

        // verify data doesn't change
        let mut msg_hex = [0u8; 2 * 14]; // 2 * user_input.len()
        {
            let secret = plaintext.as_bytes().unwrap();
            assert_eq!(str::from_utf8(&secret).unwrap(), "secret message");
            to_hex(&secret, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "736563726574206d657373616765"
            );
        }

        // ========== Encrypted Data ==========

        // encrypt data and get packet
        let ciphertext = plaintext.clone().encrypt(&dek).unwrap();
        let packet = ciphertext.as_packet().unwrap();
        {
            let mut msg_hex = vec![0u8; 2 * packet.len()];
            to_hex(&packet, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "d37267c5dfb5bbe9e86c427dfeaef9f04532917c70c536ec3b4a124a9726546573740000000000000000"
            );
            // encrypt again for second nonce
            let second_ciphertext = plaintext.clone().encrypt(&dek).unwrap();
            let packet = second_ciphertext.as_packet().unwrap();
            to_hex(&packet, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "ca9f398ae997cee3460bccf7bb4b73bd57c105b4b10b2d3ba242c5b51f45546573740000000000000001"
            );
        }

        // decrypt back to plaintext like it was recieved from network
        let data = Data::from_packet(packet, "Network Secret").unwrap();
        let decrypted = data.decrypt(&dek).unwrap();

        // verify data is the same
        {
            let secret = decrypted.as_bytes().unwrap();
            assert_eq!(str::from_utf8(&secret).unwrap(), "secret message");
            to_hex(&secret, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "736563726574206d657373616765"
            );
        }

        // ========== Authenticated Data ==========
        // encrypt data and get packet
        let ciphertext = plaintext
            .clone()
            .encrypt_authenticated(&dek, TEST_ASSOCIATED_DATA)
            .unwrap();
        let packet = ciphertext.as_packet().unwrap();
        {
            let mut msg_hex = vec![0u8; 2 * packet.len()];
            to_hex(&packet, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "2098c75bbe19ac57edb3dcd061f5969414c5d8157c2c6318a2527ac50228546573740000000000000002"
            );
            // encrypt again for second nonce
            let second_ciphertext = plaintext
                .encrypt_authenticated(&dek, TEST_ASSOCIATED_DATA)
                .unwrap();
            let packet = second_ciphertext.as_packet().unwrap();
            to_hex(&packet, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "7171af9f19efec4ac3c6a07d7af39a29bd11b1fd73f30721c3f3804674ab546573740000000000000003"
            );
        }

        // decrypt back to plaintext like it was recieved from network
        let data = Data::from_packet(packet, "Network Secret").unwrap();
        let decrypted = data.decrypt_verified(&dek, TEST_ASSOCIATED_DATA).unwrap();

        // verify data is the same
        {
            let secret = decrypted.as_bytes().unwrap();
            assert_eq!(str::from_utf8(&secret).unwrap(), "secret message");
            to_hex(&secret, &mut msg_hex).unwrap();
            assert_eq!(
                str::from_utf8(&msg_hex).unwrap(),
                "736563726574206d657373616765"
            );
        }
    }
}
