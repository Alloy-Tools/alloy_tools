use std::{fs, sync::Arc, time::Duration};

use al_core::{event, Command, Event, Transport};
use al_crypto::{Monotonic, NonceTrait};
use al_net::{CommandDispatcher, ConnectionManager, Tcp};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub address: String,
    pub port: u16,
}

impl NetworkConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseConfig {
    pub server: bool,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub network: NetworkConfig,
    pub noise: NoiseConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                address: "127.0.0.1".to_string(),
                port: 7878,
            },
            noise: NoiseConfig {
                server: false,
                timeout: Some(500),
            },
        }
    }
}

impl AppConfig {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        if !path.exists() {
            // Create default config if file doesn't exist
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        } else {
            Ok(
                toml::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            )
        }
    }

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), String> {
        fs::write(
            path,
            toml::to_string_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

const CONFIG_FILE: &str = "config.toml";

#[event]
pub struct Msg(Vec<u8>);

impl Msg {
    pub fn new(vec: impl Into<Vec<u8>>) -> Self {
        Self(vec.into())
    }

    pub fn print_msg(&self) {
        match str::from_utf8(&self.0) {
            Ok(msg) => println!("{}", msg),
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }
}

pub async fn send_recv<N: NonceTrait>(
    tcp: &Tcp<N>,
    dispatcher: &CommandDispatcher<N>,
    conn_id: u64,
    connection_manager: Arc<ConnectionManager<N>>,
    cmd: Command,
) {
    // Send command to server
    if let Err(e) = tcp.send(cmd).await {
        eprintln!("Send error: {:?}", e);
        return;
    }

    // Recv back from server
    match tcp.recv().await {
        Ok(response) => {
            dispatcher
                .dispatch(conn_id, connection_manager, response)
                .await
        }
        Err(e) => eprintln!("Recv error: {:?}", e),
    }
}

#[tokio::main]
async fn main() {
    // Read config for address and if client or server
    let config = AppConfig::load(CONFIG_FILE).unwrap();

    // Setup common server/client handlers
    let mut dispatcher = CommandDispatcher::new();
    dispatcher
        .register_event(|_, _, event: Msg| async move { event.print_msg() })
        .await;
    dispatcher
        .register_command(|_, _, cmd| async move {
            if !cmd.is_event() {
                println!("Received command: {:?}", cmd)
            }
        })
        .await;

    let connection_manager = Arc::new(ConnectionManager::new());
    // if server, start server with address
    if config.noise.server {
        // Server will echo commands back
        dispatcher
            .register_command(|id, connections, cmd| async move {
                if let Some(conn) = connections.get(id).await {
                    if let Err(e) = conn.send(cmd).await {
                        eprintln!("Error: {:?}", e)
                    }
                }
            })
            .await;

        let token = Arc::new(RwLock::new(false));
        let token_clone = token.clone();

        println!("Starting Server.");
        let server_handle = tokio::spawn(async move {
            Tcp::<Monotonic>::run_server_with_shutdown(
                config.network.address(),
                config
                    .noise
                    .timeout
                    .map(|milliseconds| Duration::from_millis(milliseconds as u64)),
                al_net::HandshakePattern::NN,
                Vec::new(),
                None,
                None,
                token_clone,
                |tcp| {
                    let dispatcher_clone = dispatcher.clone();
                    let connection_manager_clone = connection_manager.clone();
                    tokio::spawn(async move {
                        let tcp = Arc::new(tcp);
                        let conn_id = connection_manager_clone.insert(tcp.clone()).await;

                        loop {
                            tokio::task::yield_now().await;

                            match tcp.recv().await {
                                Ok(cmd) => {
                                    dispatcher_clone
                                        .dispatch(conn_id, connection_manager_clone.clone(), cmd)
                                        .await
                                }
                                Err(e) => eprintln!("Error: {:?}", e),
                            }
                        }
                    });
                },
            )
            .await
        });

        //TODO: get cancelation signal
        match server_handle.await {
            Ok(r) => {
                if let Err(e) = r {
                    eprintln!("Error: {:?}", e)
                }
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
        println!("Server Ended.");
    } else {
        println!("Attempting to connect to server.");
        // if client connect to server with address
        let tcp = Arc::new(
            Tcp::<Monotonic>::connect(
                config.network.address(),
                config
                    .noise
                    .timeout
                    .map(|milliseconds| Duration::from_millis(milliseconds as u64)),
                al_net::HandshakePattern::NN,
                Vec::new(),
                None,
                None,
            )
            .await
            .expect("Failed to connect to server."),
        );
        let conn_id = connection_manager.insert(tcp.clone()).await;
        println!("Connected to server.");

        // Send command to server and recv it back
        send_recv(
            &tcp,
            &dispatcher,
            conn_id,
            connection_manager.clone(),
            Command::Pulse,
        )
        .await;

        // Send messages to server
        send_recv(
            &tcp,
            &dispatcher,
            conn_id,
            connection_manager.clone(),
            Msg("Hello!".as_bytes().to_vec()).to_cmd(),
        )
        .await;
        println!("Client Ended.");
    }
}
