use std::sync::Arc;
use al_core::{Command, Transport};
use al_crypto::NonceTrait;
use crate::{Tcp, TcpError};

// TODO: could instead use a `Link` transport to create a transport pipeline: producer -> incoming_tcp -> outgoing_tcp -> consumer
// TODO: router should handle communication both ways? Thats why its different from a `Link` transport? or should it use two links? one for both directions?
/// A router that can forward any `Command` between `Tcp<N>` connections
pub struct Router<N: NonceTrait> {
    incoming: Arc<Tcp<N>>,
    outgoing: Arc<Tcp<N>>,
}

impl<N: NonceTrait> Router<N> {
    pub fn new(incoming: Arc<Tcp<N>>, outgoing: Arc<Tcp<N>>) -> Self {
        Self { incoming, outgoing }
    }

    /// Route commands indefinitely
    pub async fn route_commands(&mut self) -> Result<(), TcpError> {
        loop {
            // Receive a command (could be an event or any other command)
            let command: Command = self.incoming.recv().await?;

            // Log whats being routed
            match command.event_type_name() {
                Some(event_type) => println!("Routing event: {}", event_type),
                None => println!("Routing command: {:?}", command),
            }

            // Forward to outgoing connection
            self.outgoing.send(command).await?;
        }
    }
}