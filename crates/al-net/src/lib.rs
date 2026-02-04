/*
    - [x] Make a connection initiator that takes a handshake pattern
    - [] Will take an address, handle the handshake, and return the ephemeral session key and transport
    - [] Be able to send `Data<S: ProtectedState>`, or recv it and proccess it.
    - [] The data can now be any `Vec<u8>` (anything) encrypted. Send and handle `al-core` commands.
    - [] Setup simple TCP VOIP with `al-core` events.
*/

// Following the Noise protocol specification: noiseprotocol.org/noise.html

const KEY_SIZE: usize = al_crypto::KEY_SIZE;
const DOUBLE_KEY_SIZE: usize = 2 * KEY_SIZE;
const TRIPLE_KEY_SIZE: usize = 3 * KEY_SIZE;
const DHLEN: usize = al_crypto::DHLEN; // Must be 32 or greater
const HASHLEN: usize = 32; // Noise has HASHLEN 32 for BLAKE2s
const MAX_MSG_BYTE_LEN: usize = 65535; // Noise message sizes are capped at 65,535 bytes

mod command_dispatcher;
mod noise;
mod router;
mod tcp;
mod udp;
mod connection_manager;

pub use command_dispatcher::CommandDispatcher;
pub use noise::{
    cipher_state::{CipherState, CipherStateReturn},
    handshake_pattern::{HandshakePattern, HandshakeToken},
    handshake_state::{HandshakeResult, HandshakeState},
    key_pair::{KeyPair, PublicKey},
    noise_builder::NoiseBuilder as Noise,
    noise_error::NoiseError,
    symmetric_state::{SplitResult, SymmetricState},
};
pub use router::Router;
pub use tcp::{tcp::Tcp, tcp_error::TcpError};
pub use udp::udp::UDP;
pub use connection_manager::ConnectionManager;

/// A wrapper to hold any serialized `dyn Event` data for transport without needing the inner type
#[al_core::event]
pub struct NetworkEvent {
    type_name: String,
    data: Vec<u8>,
}

impl NetworkEvent {
    pub fn new<F: al_core::SerdeFormat>(event: &dyn al_core::Event) -> Result<Self, F::Error> {
        Ok(Self {
            type_name: event.type_with_generics(),
            data: F::default().serialize_event(event)?,
        })
    }

    pub fn to_inner<F: al_core::SerdeFormat>(&self) -> Result<Box<dyn al_core::Event>, F::Error> {
        Ok(F::default().deserialize_event_dyn(&self.data)?)
    }
}
