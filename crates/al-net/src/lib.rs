#[cfg(feature = "tcp")]
mod tcp;
//mod command_dispatcher;
//mod connection_manager;
//mod router;
//mod udp;

/*pub use al_secure::noise::{
    cipher_state::{CipherState, CipherStateReturn},
    handshake_pattern::{HandshakePattern, HandshakeToken},
    handshake_state::{HandshakeResult, HandshakeState},
    symmetric_state::{SplitResult, SymmetricState},
    KeyPair, Noise, NoiseError, PublicKey,
};*/
//pub use command_dispatcher::CommandDispatcher;
//pub use connection_manager::ConnectionManager;
//pub use router::Router;
//pub use tcp::{tcp::Tcp, tcp_error::TcpError};
//pub use udp::udp::UDP;

use al_events::{event, DynMessage, MessageError};
use al_structures::serde_utils::serde_registries::FormatId;

/// A wrapper to hold any serialized `dyn Event` data for transport without needing the inner type
#[event] //TODO: Should this not be marked as any type of `#[variant]` and just hold the data?
pub struct NetworkEvent {
    data: Vec<u8>,
}

impl NetworkEvent {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn with_format(format_id: FormatId, message: &DynMessage) -> Result<Self, MessageError> {
        let mut data = Vec::new();
        message.to_format(format_id, &mut data)?;
        Ok(Self { data })
    }

    pub fn to_inner(&self) -> Result<DynMessage, MessageError> {
        DynMessage::from_format_slice(&self.data)
    }
}
