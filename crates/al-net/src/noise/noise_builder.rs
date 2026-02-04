use crate::{HandshakePattern, HandshakeState, KeyPair, NoiseError, PublicKey};
use al_crypto::NonceTrait;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoiseBuilder {
    pattern: HandshakePattern,
    prologue: Vec<u8>,
    local_static: Option<KeyPair>,
    local_ephemeral: Option<KeyPair>,
    remote_static: Option<PublicKey>,
}

impl NoiseBuilder {
    pub fn new(pattern: HandshakePattern) -> Self {
        Self {
            pattern,
            prologue: Vec::new(),
            local_static: None,
            local_ephemeral: None,
            remote_static: None,
        }
    }

    pub fn with_prologue(mut self, prologue: Vec<u8>) -> Self {
        self.prologue = prologue;
        self
    }

    pub fn with_local_static(mut self, static_key: KeyPair) -> Self {
        self.local_static = Some(static_key);
        self
    }

    pub fn with_local_ephemeral(mut self, ephemeral_key: KeyPair) -> Self {
        self.local_ephemeral = Some(ephemeral_key);
        self
    }

    pub fn with_remote_static(mut self, static_key: PublicKey) -> Self {
        self.remote_static = Some(static_key);
        self
    }

    pub fn local_static(mut self, static_key: Option<KeyPair>) -> Self {
        self.local_static = static_key;
        self
    }

    pub fn local_ephemeral(mut self, ephemeral_key: Option<KeyPair>) -> Self {
        self.local_ephemeral = ephemeral_key;
        self
    }

    pub fn remote_static(mut self, static_key: Option<PublicKey>) -> Self {
        self.remote_static = static_key;
        self
    }

    fn build<N: NonceTrait>(self, initiator: bool) -> Result<HandshakeState<N>, NoiseError> {
        HandshakeState::initialize(
            self.pattern,
            initiator,
            &self.prologue,
            self.local_static,
            self.local_ephemeral,
            self.remote_static,
            None,
        )
    }

    pub fn initiate<N: NonceTrait>(self) -> Result<HandshakeState<N>, NoiseError> {
        Self::build(self, true)
    }

    pub fn respond<N: NonceTrait>(self) -> Result<HandshakeState<N>, NoiseError> {
        Self::build(self, false)
    }
}
