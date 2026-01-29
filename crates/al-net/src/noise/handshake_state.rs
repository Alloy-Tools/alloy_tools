use al_crypto::diffie_hellman;
use al_vault::SecureAccess;
use zeroize::Zeroize;

use crate::{
    DHLEN, HandshakePattern, HandshakeToken, KeyPair, NoiseError, PublicKey, SplitResult, SymmetricState
};

pub struct HandshakeState {
    symmetric_state: SymmetricState,
    s: Option<KeyPair>,
    e: Option<KeyPair>,
    rs: Option<PublicKey>,
    re: Option<PublicKey>,
    initiator: bool,
    message_patterns: Vec<Vec<HandshakeToken>>,
}

impl HandshakeState {
    pub fn initialize(
        pattern: HandshakePattern,
        initiator: bool,
        prologue: &[u8],
        s: Option<KeyPair>,
        e: Option<KeyPair>,
        rs: Option<PublicKey>,
        re: Option<PublicKey>,
    ) -> Result<Self, NoiseError> {
        let mut symmetric_state = SymmetricState::initialize_symmetric(pattern.clone());
        symmetric_state.mix_hash(prologue);
        pattern.mix_premessages(&mut symmetric_state, initiator, &s, &e, &rs, &re)?;
        Ok(Self {
            symmetric_state,
            s,
            e,
            rs,
            re,
            initiator,
            message_patterns: pattern.to_tokens(),
        })
    }

    pub fn is_complete(&self) -> bool {
        self.message_patterns.is_empty()
    }

    pub fn is_initiator(&self) -> bool {
        self.initiator
    }

    fn try_diffie_hellman(key_pair: &Option<KeyPair>, public_key: &Option<PublicKey>, key_pair_missing_error: NoiseError, public_key_missing_error: NoiseError) -> Result<[u8; DHLEN], NoiseError> {
        match (key_pair, public_key) {
            (Some(key_pair), Some(public)) => {
                // Calls DH(key_pair.private, public_key).
                Ok(key_pair.private().with(|private| diffie_hellman(*private, public.to_bytes())))
            },
            (Some(_), None) => Err(public_key_missing_error)?,
            (None, Some(_)) => Err(key_pair_missing_error)?,
            _ => Err(NoiseError::BothKeysMissing)?,
        }
    }

    fn try_mix_key<F: FnOnce(&Self) -> (&Option<KeyPair>, &Option<PublicKey>, NoiseError, NoiseError)>(&mut self, f: F) -> Result<(), NoiseError> {
        let (key_pair, public_key, key_pair_missing_error, public_key_missing_error) = f(self);
        let mut dh = Self::try_diffie_hellman(key_pair, public_key, key_pair_missing_error, public_key_missing_error)?;
        let result = self.symmetric_state.mix_key(&dh);
        dh.zeroize();
        result
    }

    /// Message buffer can be `[0u8; 65535]` to prevent reallocation as that is the max Noise message size
    pub fn write_message(
        &mut self,
        payload: &mut [u8],
        message_buffer: &mut [u8],
    ) -> Result<Option<SplitResult>, NoiseError> {
        if self.is_complete() {
            Err(NoiseError::HandshakeComplete)?
        }

        let mut head = 0;
        for token in self.message_patterns.remove(0) {
            head += self.process_write(token, message_buffer, head)?
        }

        let ciphertext = self.symmetric_state.encrypt_and_hash(payload)?;
        message_buffer[head..head + ciphertext.len()].copy_from_slice(&ciphertext);

        if self.is_complete() {
            Ok(Some(self.symmetric_state.split()?))
        } else {
            Ok(None)
        }
    }

    /// Performs actions and writes bytes according to the token, returns the number of bytes written
    fn process_write(
        &mut self,
        token: HandshakeToken,
        message_buffer: &mut [u8],
        head: usize,
    ) -> Result<usize, NoiseError> {
        let mut len = 0;
        match token {
            HandshakeToken::E => {
                // Sets e (which must be empty) to GENERATE_KEYPAIR().
                if self.e.is_some() { Err(NoiseError::LocalEphemeralExists)? }
                let pair = KeyPair::new();

                // Appends e.public_key to the buffer.
                let mut public_bytes = pair.public().to_bytes();
                self.e = Some(pair);
                len = public_bytes.len();
                message_buffer[head..head + len].copy_from_slice(&public_bytes);

                // Calls MixHash(e.public_key).
                self.symmetric_state.mix_hash(&public_bytes);
                public_bytes.zeroize();
            },
            HandshakeToken::S => {
                match &self.s {
                    Some(key_pair) => {
                        // Appends EncryptAndHash(s.public_key) to the buffer.
                        let mut s_pub = key_pair.public().to_bytes();
                        let ciphertext = self.symmetric_state.encrypt_and_hash(&mut s_pub)?;
                        len = ciphertext.len();
                        message_buffer[head..head + len].copy_from_slice(&ciphertext);
                    },
                    None => Err(NoiseError::LocalStaticMissing)?,
                }
            },
            HandshakeToken::EE => {
                // Calls MixKey(DH(e, re)).
                self.try_mix_key(|hs| (&hs.e, &hs.re, NoiseError::LocalEphemeralMissing, NoiseError::RemoteEphemeralMissing))?
            },
            HandshakeToken::ES => {
                // Calls MixKey(DH(e, rs)) if initiator.
                if self.initiator {
                    self.try_mix_key(|hs| (&hs.e, &hs.rs, NoiseError::LocalEphemeralMissing, NoiseError::RemoteStaticMissing))?
                } else {// Calls MixKey(DH(s, re)) if responder.
                    self.try_mix_key(|hs| (&hs.s, &hs.re, NoiseError::LocalStaticMissing, NoiseError::RemoteEphemeralMissing))?
                }
            },
            HandshakeToken::SE => {
                // Calls MixKey(DH(s, re)) if initiator.
                if self.initiator {
                    self.try_mix_key(|hs| (&hs.s, &hs.re, NoiseError::LocalStaticMissing, NoiseError::RemoteEphemeralMissing))?
                } else { // Calls MixKey(DH(e, rs)) if responder.
                    self.try_mix_key(|hs| (&hs.e, &hs.rs, NoiseError::LocalEphemeralMissing, NoiseError::RemoteStaticMissing))?
                }
            },
            HandshakeToken::SS => {
                // Calls MixKey(DH(s, rs)).
                self.try_mix_key(|hs| (&hs.s, &hs.rs, NoiseError::LocalStaticMissing, NoiseError::RemoteStaticMissing))?
            },
            HandshakeToken::PSK => todo!(),
        }
        Ok(len)
    }

    /// Payload buffer can be `[0u8; 65535]` as that is the max Noise message size
    pub fn read_message(
        &mut self,
        message: &mut [u8],
        payload_buffer: &mut [u8],
    ) -> Result<Option<SplitResult>, NoiseError> {
        if self.is_complete() {
            Err(NoiseError::HandshakeComplete)?
        }

        let mut head = 0;
        for token in self.message_patterns.remove(0) {
            head += self.process_read(token, message, head)?
        }

        // Call DecryptAndHash() on the remaining bytes of the message and store the output into payload_buffer.
        if message.len() - head > 0 {
            let plaintext = self
                .symmetric_state
                .decrypt_and_hash(message[head..].as_mut())?;
            let len = plaintext.len().min(payload_buffer.len());
            payload_buffer[..len].copy_from_slice(&plaintext[..len]);
        }

        if self.is_complete() {
            Ok(Some(self.symmetric_state.split()?))
        } else {
            Ok(None)
        }
    }

    /// Reads bytes and performs actions according to the token, returns the number of bytes read
    fn process_read(
        &mut self,
        token: HandshakeToken,
        message: &[u8],
        head: usize,
    ) -> Result<usize, NoiseError> {
        let mut len = 0;
        match token {
            HandshakeToken::E => {
                // Sets re (which must be empty) to the next DHLEN bytes from the message.
                if self.re.is_some() { Err(NoiseError::RemoteEphemeralExists)? }
                len = DHLEN;
                let public_bytes = &message[head..head + DHLEN];
                self.re = Some(PublicKey::from_bytes(public_bytes)?);

                //Calls MixHash(re.public_key).
                self.symmetric_state.mix_hash(public_bytes);
            },
            HandshakeToken::S => {
                if self.rs.is_some() { Err(NoiseError::RemoteStaticExists)? }

                // Sets temp to the next DHLEN + 16 bytes of the message if HasKey() == True, or to the next DHLEN bytes otherwise.
                let mut temp = [0u8; DHLEN + 16];
                len = DHLEN + if self.symmetric_state.has_key() {16} else {0};
                temp[..len].copy_from_slice(&message[head..head + len]);

                // Sets rs (which must be empty) to DecryptAndHash(temp).
                let mut bytes = self.symmetric_state.decrypt_and_hash(&mut temp[..len])?;
                self.rs = match PublicKey::from_bytes(&bytes) {
                    Ok(public) => Ok(Some(public)),
                    Err(e) => {
                        bytes.zeroize();
                        Err(e)
                    },
                }?;
            },
            HandshakeToken::EE => {
                // Calls MixKey(DH(e, re)).
                self.try_mix_key(|hs| (&hs.e, &hs.re, NoiseError::LocalEphemeralMissing, NoiseError::RemoteEphemeralMissing))?
            },
            HandshakeToken::ES => {
                // Calls MixKey(DH(e, rs)) if initiator.
                if self.initiator {
                    self.try_mix_key(|hs| (&hs.e, &hs.rs, NoiseError::LocalEphemeralMissing, NoiseError::RemoteStaticMissing))?
                } else { // Calls MixKey(DH(s, re)) if responder.
                    self.try_mix_key(|hs| (&hs.s, &hs.re, NoiseError::LocalStaticMissing, NoiseError::RemoteEphemeralMissing))?
                }
            },
            HandshakeToken::SE => {
                // Calls MixKey(DH(s, re)) if initiator.
                if self.initiator {
                    self.try_mix_key(|hs| (&hs.s, &hs.re, NoiseError::LocalStaticMissing, NoiseError::RemoteEphemeralMissing))?
                } else { // Calls MixKey(DH(e, rs)) if responder.
                    self.try_mix_key(|hs| (&hs.e, &hs.rs, NoiseError::LocalEphemeralMissing, NoiseError::RemoteStaticMissing))?
                }
            },
            HandshakeToken::SS => {
                // Calls MixKey(DH(s, rs)).
                self.try_mix_key(|hs| (&hs.s, &hs.rs, NoiseError::LocalStaticMissing, NoiseError::RemoteStaticMissing))?
            },
            HandshakeToken::PSK => todo!(),
        }
        Ok(len)
    }
}
