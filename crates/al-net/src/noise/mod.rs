pub mod cipher_state;
pub mod handshake_pattern;
pub mod handshake_state;
pub mod key_pair;
pub mod noise_builder;
pub mod noise_error;
pub mod symmetric_state;

#[cfg(test)]
mod tests {
    use al_crypto::{Microseconds, Monotonic, RandomTimeStamp};
    use al_vault::SecureAccess;
    use zeroize::Zeroize;

    use crate::{HandshakeResult, KeyPair, Noise, PublicKey, KEY_SIZE, MAX_MSG_BYTE_LEN};

    const TEST_MSG: &[u8; 14] = b"secret message";
    const TEST_STATIC_I: [u8; 32] = [
        54, 204, 226, 149, 59, 170, 202, 179, 39, 51, 78, 144, 190, 98, 38, 222, 177, 244, 71, 48,
        232, 63, 157, 99, 137, 117, 121, 51, 144, 223, 137, 130,
    ];
    const TEST_STATIC_I_PUB: [u8; 32] = [
        167, 1, 217, 60, 243, 178, 129, 109, 174, 99, 120, 54, 173, 205, 101, 4, 84, 98, 199, 118,
        153, 184, 85, 95, 179, 160, 172, 182, 33, 122, 100, 122,
    ];
    const TEST_STATIC_R: [u8; 32] = [
        152, 120, 43, 42, 37, 109, 46, 119, 178, 204, 89, 29, 45, 109, 174, 126, 253, 212, 208,
        237, 90, 127, 112, 2, 195, 224, 225, 151, 222, 97, 118, 154,
    ];
    const TEST_STATIC_R_PUB: [u8; 32] = [
        48, 203, 114, 127, 182, 56, 24, 179, 60, 87, 240, 145, 136, 107, 230, 212, 151, 154, 4,
        235, 104, 35, 131, 93, 247, 14, 98, 74, 152, 206, 183, 1,
    ];

    #[test]
    fn nn_handshake() {
        let mut initiator = Noise::new(crate::HandshakePattern::NN)
            .initiate::<RandomTimeStamp<Microseconds>>()
            .unwrap();

        let mut responder = Noise::new(crate::HandshakePattern::NN)
            .respond::<RandomTimeStamp<Microseconds>>()
            .unwrap();

        let mut initiator_buffer = [0u8; MAX_MSG_BYTE_LEN];
        let mut responder_buffer = [0u8; MAX_MSG_BYTE_LEN];

        // ============= Message 1 =============

        // Initiator writes ephemeral to buffer, which will then be sent
        match initiator.write_message(&mut [], &mut initiator_buffer) {
            Ok(HandshakeResult::InProgress(len)) => assert_eq!(len, KEY_SIZE as u16),
            _ => panic!("Handshake still ongoing!"),
        }

        // Responder reads message from initiator
        match responder.read_message(&mut initiator_buffer[..KEY_SIZE], &mut responder_buffer) {
            Ok(HandshakeResult::InProgress(len)) => assert_eq!(len, 0),
            _ => panic!("Handshake still ongoing!"),
        }

        // Assert ephemeral was sent correctly
        assert_eq!(
            initiator.e.as_ref().and_then(|e| Some(e.public())),
            responder.re.as_ref()
        );

        // Assert hashes are equal
        initiator
            .symmetric_state
            .h
            .with(|i_h| responder.symmetric_state.h.with(|r_h| assert_eq!(i_h, r_h)));

        // Assert both buffers are empty
        assert!(initiator_buffer.iter().all(|b| *b == 0));
        assert!(responder_buffer.iter().all(|b| *b == 0));

        // ============= Message 2 =============

        // Responder writes ephemeral into buffer, which will then be sent, and calculates DH(resp_e, init_e)
        let responder_split = match responder.write_message(&mut [], &mut responder_buffer) {
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                // Should be 32 for the responder e public, 16 for empty encrypt tag, 12 for nonce = 60
                assert_eq!(len, 60);
                (init, resp, handshake_hash, len)
            }
            _ => panic!("Responder handshake completed!"),
        };

        // Initiator reads 32 byte key, calculates DH(init_e, resp_e), then reads and decrypts the 28 byte ciphertext from the buffer
        let initiator_split = match initiator.read_message(
            &mut responder_buffer[..responder_split.3 as usize],
            &mut initiator_buffer,
        ) {
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                assert_eq!(len, 0);
                (init, resp, handshake_hash)
            }
            Err(e) => panic!("Error: {:?}", e),
            _ => panic!("Initiator handshake completed!"),
        };

        // Assert ephemeral was sent correctly
        assert_eq!(
            responder.e.as_ref().and_then(|e| Some(e.public())),
            initiator.re.as_ref()
        );

        // Assert hashes are equal
        initiator
            .symmetric_state
            .h
            .with(|i_h| responder.symmetric_state.h.with(|r_h| assert_eq!(i_h, r_h)));

        // Assert both buffers are empty
        assert!(initiator_buffer.iter().all(|b| *b == 0));
        assert!(responder_buffer.iter().all(|b| *b == 0));

        // ============= Completed =============

        // Encrypt a message to send
        let mut plaintext = [0u8; 14];
        plaintext.copy_from_slice(TEST_MSG);
        let mut msg = initiator_split
            .0
            .encrypt_with_ad(&[], &mut plaintext)
            .unwrap();
        plaintext.zeroize();

        // Decrypt sent message
        plaintext.copy_from_slice(
            &responder_split
                .0
                .decrypt_with_ad(&[], msg.as_mut_slice())
                .unwrap(),
        );
        assert_eq!(&plaintext, TEST_MSG);
    }

    #[test]
    fn kk_handshake() {
        let mut initiator = Noise::new(crate::HandshakePattern::KK)
            .with_local_static(KeyPair::from_bytes(TEST_STATIC_I))
            .with_remote_static(PublicKey::from_bytes(&TEST_STATIC_R_PUB).unwrap())
            .initiate::<Monotonic>()
            .unwrap();

        let mut responder = Noise::new(crate::HandshakePattern::KK)
            .with_local_static(KeyPair::from_bytes(TEST_STATIC_R))
            .with_remote_static(PublicKey::from_bytes(&TEST_STATIC_I_PUB).unwrap())
            .respond::<Monotonic>()
            .unwrap();

        let mut initiator_buffer = [0u8; MAX_MSG_BYTE_LEN];
        let mut responder_buffer = [0u8; MAX_MSG_BYTE_LEN];

        // ============= Message 1 =============

        // Initiator writes ephemeral to buffer, which will then be sent, and calculates DH(init_e, resp_s) and DH(init_s, resp_s)
        let msg1_len = match initiator.write_message(&mut [], &mut initiator_buffer) {
            Ok(HandshakeResult::InProgress(len)) => {
                // Should be 32 for the responder e public, 16 for empty encrypt tag, 12 for nonce = 60
                assert_eq!(len, 60);
                len
            }
            _ => panic!("Handshake still ongoing!"),
        };

        // Responder reads ephemeral from initiator and calculates DH(resp_s, init_e) and DH(resp_s, init_s)
        match responder.read_message(&mut initiator_buffer[..msg1_len as usize], &mut responder_buffer) {
            Ok(HandshakeResult::InProgress(len)) => assert_eq!(len, 0),
            _ => panic!("Handshake still ongoing!"),
        }

        // Assert ephemeral was correctly sent
        assert_eq!(
            initiator.e.as_ref().and_then(|e| Some(e.public())),
            responder.re.as_ref()
        );

        // Assert hashes are equal
        initiator
            .symmetric_state
            .h
            .with(|i_h| responder.symmetric_state.h.with(|r_h| assert_eq!(i_h, r_h)));

        assert!(initiator_buffer.iter().all(|b| *b == 0));
        assert!(responder_buffer.iter().all(|b| *b == 0));

        // ============= Message 2 =============

        // Responder writes ephemeral into buffer, which will then be sent, and calculates DH(resp_e, init_e) and DH(resp_e, init_s)
        let responder_split = match responder.write_message(&mut [], &mut responder_buffer) {
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                // Should be 32 for the responder e public, 16 for empty encrypt tag, 12 for nonce = 60
                assert_eq!(len, 60);
                (init, resp, handshake_hash, len)
            }
            _ => panic!("Responder handshake completed!"),
        };

        // Initiator reads 32 byte key, calculates DH(init_e, resp_e) and DH(init_s, resp_e), then read and decrypts the 28 byte ciphertext from the buffer
        let initiator_split = match initiator.read_message(
            &mut responder_buffer[..responder_split.3 as usize],
            &mut initiator_buffer,
        ) {
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                assert_eq!(len, 0);
                (init, resp, handshake_hash)
            }
            Err(e) => panic!("Error: {:?}", e),
            _ => panic!("Initiator handshake completed!"),
        };

        // Assert ephemeral was sent correctly
        assert_eq!(
            responder.e.as_ref().and_then(|e| Some(e.public())),
            initiator.re.as_ref()
        );

        // Assert hashes are equal
        initiator
            .symmetric_state
            .h
            .with(|i_h| responder.symmetric_state.h.with(|r_h| assert_eq!(i_h, r_h)));

        // Assert both buffers are empty
        assert!(initiator_buffer.iter().all(|b| *b == 0));
        assert!(responder_buffer.iter().all(|b| *b == 0));

        // ============= Completed =============

        // Encrypt a message to send
        let mut plaintext = [0u8; 14];
        plaintext.copy_from_slice(TEST_MSG);
        let mut msg = initiator_split
            .0
            .encrypt_with_ad(&[], &mut plaintext)
            .unwrap();
        plaintext.zeroize();

        // Decrypt sent message
        plaintext.copy_from_slice(
            &responder_split
                .0
                .decrypt_with_ad(&[], msg.as_mut_slice())
                .unwrap(),
        );
        assert_eq!(&plaintext, TEST_MSG);
    }
}
