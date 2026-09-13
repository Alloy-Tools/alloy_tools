/*
    - [x] Make a connection initiator that takes a handshake pattern
    - [x] Will take an address, handle the handshake, and return the ephemeral session key and transport
    - [] Be able to send `Data<S: ProtectedState>`, or recv it and proccess it.
    - [x] The data can now be any `Vec<u8>` (anything) encrypted. Send and handle `al-core` commands.
    - [] Setup simple TCP VOIP with `al-core` events.
*/

/*mod command_dispatcher;
mod connection_manager;
mod router;
mod tcp;
mod udp;*/

/*pub use al_secure::noise::{
    cipher_state::{CipherState, CipherStateReturn},
    handshake_pattern::{HandshakePattern, HandshakeToken},
    handshake_state::{HandshakeResult, HandshakeState},
    symmetric_state::{SplitResult, SymmetricState},
    KeyPair, Noise, NoiseError, PublicKey,
};
pub use command_dispatcher::CommandDispatcher;
pub use connection_manager::ConnectionManager;
pub use router::Router;
pub use tcp::{tcp::Tcp, tcp_error::TcpError};
pub use udp::udp::UDP;*/

use al_events::{event, DynMessage, IdCache, MESSAGE_FORMATS, MESSAGE_TYPE_REGISTRY};
use al_structures::serde_utils::serde_registries::FormatId;

/// A wrapper to hold any serialized `dyn Event` data for transport without needing the inner type
#[event]
pub struct NetworkEvent {
    data: Vec<u8>,
}

impl NetworkEvent {
    pub fn new(
        format_id: FormatId,
        message: &DynMessage,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut data = Vec::new();
        let type_id = message.message_id()?;
        MESSAGE_FORMATS().serialize_registered(
            MESSAGE_TYPE_REGISTRY(),
            format_id,
            type_id,
            message,
            &mut data,
        )?;
        Ok(Self { data })
    }

    /*pub fn to_inner<F: al_core::SerdeFormat>(&self) -> Result<Box<dyn al_core::Event>, F::Error> {
        Ok(F::default().deserialize_event_dyn(&self.data)?)
    }*/
}
