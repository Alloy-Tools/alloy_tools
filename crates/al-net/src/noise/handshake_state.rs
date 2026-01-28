use crate::{
    HandshakePattern, HandshakeToken, KeyPair, NoiseError, PublicKey, SplitResult, SymmetricState,
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

    fn process_write(
        &mut self,
        token: HandshakeToken,
        message_buffer: &mut [u8],
        head: usize,
    ) -> Result<usize, NoiseError> {
        todo!()
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
            head += self.process_read(token, payload_buffer, head)?
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

    fn process_read(
        &mut self,
        token: HandshakeToken,
        payload_buffer: &mut [u8],
        head: usize,
    ) -> Result<usize, NoiseError> {
        todo!()
    }
}
