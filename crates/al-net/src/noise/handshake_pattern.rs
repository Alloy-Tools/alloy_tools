use crate::{KeyPair, NoiseError, PublicKey, SymmetricState, HASHLEN};

//TODO: decide all handshake patterns
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum HandshakePattern {
    /// No Authentication, no static keys (anonymous)
    #[default]
    NN,
    /// Mutual static key authentication
    KK,
    /// Mutual authentication with ephemerals
    XX,
    /// Initiator knows responder static, only responder authenticated
    NK,
    /// Responder knows initiator static, only initiator authenticated
    KN,
    /// Authenticates responder, initiator can be anonymous
    XK,
    /// Authenticates initiator, responder can be anonymous
    KX,
    /// Initiator anonymous, responder transmits static but can be anonymous
    NX,
    /// Initiator transmits static but can be anonymous, responder anonymous
    XN,
}

impl HandshakePattern {
    /// Returns a two bool tuple representing `(initiator, responder)` required
    pub fn requires_premessage(&self) -> (bool, bool) {
        match self {
            HandshakePattern::NN
            | HandshakePattern::XX
            | HandshakePattern::NX
            | HandshakePattern::XN => (false, false),
            // _K: initiator knows responder static
            HandshakePattern::NK | HandshakePattern::XK => (false, true),
            // K_: responder knows initiator static
            HandshakePattern::KN | HandshakePattern::KX => (true, false),
            // KK: both keys are known, initiator key is hashed first
            HandshakePattern::KK => (true, true),
        }
    }

    /// Mixes any known static public keys as a pre-message.
    /// If both initiator and responder have pre-messages, the initiator's public keys are hashed first.
    pub fn mix_premessages(
        &self,
        symmetric_state: &mut SymmetricState,
        initiator: bool,
        s: &Option<KeyPair>,
        _e: &Option<KeyPair>,
        rs: &Option<PublicKey>,
        _re: &Option<PublicKey>,
    ) -> Result<(), NoiseError> {
        match self {
            HandshakePattern::NN
            | HandshakePattern::XX
            | HandshakePattern::NX
            | HandshakePattern::XN => {}
            // _K: initiator knows responder static
            HandshakePattern::NK | HandshakePattern::XK => match initiator {
                true => self.mix_remote(symmetric_state, rs)?,
                false => self.mix_local(symmetric_state, s)?,
            },
            // K_: responder knows initiator static
            HandshakePattern::KN | HandshakePattern::KX => match initiator {
                true => self.mix_local(symmetric_state, s)?,
                false => self.mix_remote(symmetric_state, rs)?,
            },
            // KK: both keys are known, initiator key is hashed first
            HandshakePattern::KK => {
                if initiator {
                    self.mix_local(symmetric_state, s)?
                }
                self.mix_remote(symmetric_state, rs)?;
                if !initiator {
                    self.mix_local(symmetric_state, s)?
                }
            }
        }
        Ok(())
    }

    fn mix_local(
        &self,
        symmetric_state: &mut SymmetricState,
        local: &Option<KeyPair>,
    ) -> Result<(), NoiseError> {
        match local {
            Some(local) => symmetric_state.mix_hash(local.public().as_bytes()),
            None => Err(NoiseError::LocalStaticMissing)?,
        }
        Ok(())
    }

    fn mix_remote(
        &self,
        symmetric_state: &mut SymmetricState,
        remote: &Option<PublicKey>,
    ) -> Result<(), NoiseError> {
        match remote {
            Some(remote) => symmetric_state.mix_hash(remote.as_bytes()),
            None => Err(NoiseError::RemoteStaticMissing)?,
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> [u8; HASHLEN] {
        let protocol = match self {
            HandshakePattern::NN => b"Noise_NN_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::KK => b"Noise_KK_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::XX => b"Noise_XX_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::NK => b"Noise_NK_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::KN => b"Noise_KN_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::XK => b"Noise_XK_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::KX => b"Noise_KX_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::NX => b"Noise_NX_25519_ChaChaPoly_BLAKE2s",
            HandshakePattern::XN => b"Noise_XN_25519_ChaChaPoly_BLAKE2s",
        };
        match protocol.len() {
            n if n <= HASHLEN => {
                let mut out = [0u8; HASHLEN];
                out[..n].copy_from_slice(protocol);
                out
            }
            _ => al_crypto::hash(protocol),
        }
    }

    pub fn to_tokens(&self) -> Vec<Vec<HandshakeToken>> {
        match self {
            HandshakePattern::NN => vec![
                vec![HandshakeToken::E],
                vec![HandshakeToken::E, HandshakeToken::EE],
            ],
            HandshakePattern::KK => vec![
                vec![HandshakeToken::E, HandshakeToken::ES, HandshakeToken::SS],
                vec![HandshakeToken::E, HandshakeToken::EE, HandshakeToken::SE],
            ],
            HandshakePattern::XX => vec![
                vec![HandshakeToken::E],
                vec![
                    HandshakeToken::E,
                    HandshakeToken::EE,
                    HandshakeToken::S,
                    HandshakeToken::ES,
                ],
                vec![HandshakeToken::S, HandshakeToken::SE],
            ],
            HandshakePattern::NK => vec![
                vec![HandshakeToken::E, HandshakeToken::ES],
                vec![HandshakeToken::E, HandshakeToken::EE],
            ],
            HandshakePattern::KN => vec![
                vec![HandshakeToken::E],
                vec![HandshakeToken::E, HandshakeToken::EE, HandshakeToken::SE],
            ],
            HandshakePattern::XK => vec![
                vec![HandshakeToken::E, HandshakeToken::ES],
                vec![HandshakeToken::E, HandshakeToken::EE],
                vec![HandshakeToken::S, HandshakeToken::SE],
            ],
            HandshakePattern::KX => vec![
                vec![HandshakeToken::E],
                vec![
                    HandshakeToken::E,
                    HandshakeToken::EE,
                    HandshakeToken::SE,
                    HandshakeToken::S,
                    HandshakeToken::ES,
                ],
            ],
            HandshakePattern::NX => vec![
                vec![HandshakeToken::E],
                vec![
                    HandshakeToken::E,
                    HandshakeToken::EE,
                    HandshakeToken::S,
                    HandshakeToken::ES,
                ],
            ],
            HandshakePattern::XN => vec![
                vec![HandshakeToken::E],
                vec![HandshakeToken::E, HandshakeToken::EE],
                vec![HandshakeToken::S, HandshakeToken::SE],
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandshakeToken {
    /// Ephemeral key
    E,
    /// Static key
    S,
    /// Ephemeral-ephemeral DH
    EE,
    /// Ephemeral-static DH (initiator's ephemeral with responder's static)
    ES,
    /// Static-ephemeral DH (initiator's static with responder's ephemeral)
    SE,
    /// Static-static DH
    SS,
    /// Pre-shared key
    PSK,
}
