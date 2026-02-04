use al_core::{Transport, TransportItemRequirements};
use al_crypto::NonceTrait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Mutex, RwLock},
    time::timeout as tokio_timeout,
};
use zeroize::Zeroize;

use crate::{
    CipherState, HandshakePattern, HandshakeResult, HandshakeState, KeyPair, Noise, PublicKey,
    TcpError, HASHLEN, MAX_MSG_BYTE_LEN,
};
use std::{sync::Arc, time::Duration};

pub struct Tcp<N: NonceTrait> {
    stream: Arc<Mutex<TcpStream>>,
    buffer: Mutex<[u8; MAX_MSG_BYTE_LEN]>,
    noise: HandshakeState<N>,
    split: Option<(
        Arc<RwLock<CipherState<N>>>, // Initiator sends with first
        Arc<RwLock<CipherState<N>>>, // Responder sends with second
        [u8; HASHLEN],
    )>,
    timeout: Option<Duration>,
}

impl<N: NonceTrait> std::fmt::Debug for Tcp<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tcp")
            .field("stream", &"Arc<Mutex<TcpStream>>")
            .field("buffer", &self.buffer)
            .field("noise", &self.noise)
            .field(
                "split",
                &if let Some(split) = &self.split {
                    format!(
                        "(CipherState<{:?}>, CipherState<{:?}>, {:?})",
                        N::nonce_type(),
                        N::nonce_type(),
                        split.2
                    )
                } else {
                    "None".to_string()
                },
            )
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl<N: NonceTrait> Tcp<N> {
    pub async fn connect<A: ToSocketAddrs>(
        addr: A,
        timeout: Option<Duration>,
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
    ) -> Result<Self, TcpError> {
        let attempt = TcpStream::connect(addr);

        let stream = if let Some(timeout) = timeout {
            tokio_timeout(timeout, attempt).await?
        } else {
            attempt.await
        }?;

        let noise = Noise::new(pattern)
            .local_static(local_static)
            .remote_static(remote_static)
            .with_prologue(prologue)
            .initiate()?;

        let mut tcp = Self {
            stream: Arc::new(Mutex::new(stream)),
            buffer: Mutex::new([0u8; MAX_MSG_BYTE_LEN]),
            noise,
            split: None,
            timeout,
        };
        tcp.handle_handshake().await?;

        Ok(tcp)
    }

    pub async fn from_stream(
        stream: TcpStream,
        timeout: Option<Duration>,
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
    ) -> Result<Self, TcpError> {
        let noise = Noise::new(pattern)
            .local_static(local_static)
            .remote_static(remote_static)
            .with_prologue(prologue)
            .respond()?;

        let mut tcp = Self {
            stream: Arc::new(Mutex::new(stream)),
            buffer: Mutex::new([0u8; MAX_MSG_BYTE_LEN]),
            noise,
            split: None,
            timeout,
        };
        tcp.handle_handshake().await?;

        Ok(tcp)
    }

    pub async fn create_listener<A: ToSocketAddrs>(addr: A) -> Result<TcpListener, TcpError> {
        Ok(TcpListener::bind(addr).await?)
    }

    /// Creates a listener for the specified address and infinitely accepts clients, calling the passed closure with the connection after completing the handshake.
    pub async fn run_server<A: ToSocketAddrs, F: Fn(Tcp<N>)>(
        addr: A,
        timeout: Option<Duration>,
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
        f: F,
    ) -> Result<(), TcpError> {
        let listener = Self::create_listener(addr).await?;

        loop {
            // Accept connections with handshake
            match Self::accept_connection(
                &listener,
                &timeout,
                &pattern,
                &prologue,
                &local_static,
                &remote_static,
            )
            .await
            {
                // Call passed closure
                Ok(conn) => f(conn),
                // Sleep on timeout to avoid a tight loop
                Err(TcpError::Timeout(_)) => tokio::time::sleep(Duration::from_millis(10)).await,
                Err(e) => eprintln!("Failed to accept connection: {:?}", e),
            }
        }
    }

    /// Creates a server that checks the passed bool token for a shutdown signal. Requires `timeout` to be `Some(Duration)` to allow the server to loop and check the signal.
    pub async fn run_server_with_shutdown<A: ToSocketAddrs, F: Fn(Tcp<N>)>(
        addr: A,
        timeout: Option<Duration>,
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
        f: F,
        token: Arc<RwLock<bool>>,
    ) -> Result<(), TcpError> {
        let listener = Self::create_listener(addr).await?;

        loop {
            // Stop server if canceled
            if *token.read().await {
                break Ok(());
            }

            // Accept connections with handshake
            match Self::accept_connection(
                &listener,
                &timeout,
                &pattern,
                &prologue,
                &local_static,
                &remote_static,
            )
            .await
            {
                // Call passed closure
                Ok(conn) => f(conn),
                // Sleep on timeout to avoid a tight loop
                Err(TcpError::Timeout(_)) => tokio::time::sleep(Duration::from_millis(10)).await,
                Err(e) => eprintln!("Failed to accept connection: {:?}", e),
            }
        }
    }

    /// Accepts a connection from the passed listener, handling the handshake and returning a `Tcp` struct
    pub async fn accept_connection(
        listener: &TcpListener,
        timeout: &Option<Duration>,
        pattern: &HandshakePattern,
        prologue: &Vec<u8>,
        local_static: &Option<KeyPair>,
        remote_static: &Option<PublicKey>,
    ) -> Result<Self, TcpError> {
        // Accept a connection, possibly with timeout
        let future = async { listener.accept().await };
        let (stream, _) = if let Some(timeout) = timeout {
            tokio_timeout(*timeout, future).await??
        } else {
            future.await?
        };

        // Create and return a Tcp stuct from the stream
        Self::from_stream(
            stream,
            timeout.clone(),
            pattern.clone(),
            prologue.clone(),
            local_static.clone(),
            remote_static.clone(),
        )
        .await
    }

    /// Handles writing the current Noise step and payload to buffer, then sending the buffer
    async fn write_message(&mut self, payload: &mut [u8]) -> Result<(), TcpError> {
        let mut buffer = self.buffer.lock().await;
        // Write noise message to buffer
        let len = match self.noise.write_message(payload, &mut *buffer) {
            Ok(HandshakeResult::InProgress(len)) => len,
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                self.split = Some((
                    Arc::new(RwLock::new(init)),
                    Arc::new(RwLock::new(resp)),
                    handshake_hash,
                ));
                len
            }
            Err(e) => Err(e)?,
        };

        // Closure to send length prefixed message
        let mut future = async || -> Result<(), TcpError> {
            let mut stream = self.stream.lock().await;
            stream.write(&len.to_be_bytes()).await?;
            let msg = &mut buffer[..len as usize];
            stream.write_all(msg).await?;
            msg.zeroize();
            Ok(())
        };

        // Possibly stop closure after timeout
        if let Some(timeout) = self.timeout {
            tokio_timeout(timeout, future()).await?
        } else {
            future().await
        }
    }

    /// Handles reading the current noise step and payload to buffer. Returns the length of the payload in the buffer
    async fn read_message(&mut self, payload_buffer: &mut [u8]) -> Result<u16, TcpError> {
        let mut buffer = self.buffer.lock().await;
        let mut recv_len = [0u8; 2];

        // Closure to recv message into buffer, reading length first
        let mut future = async || -> Result<usize, TcpError> {
            let mut stream = self.stream.lock().await;
            stream.read_exact(&mut recv_len).await?;
            let len = u16::from_be_bytes(recv_len) as usize;
            stream.read_exact(&mut buffer[..len]).await?;
            Ok(len)
        };

        // Possibly stop closure after timeout
        let len = if let Some(timeout) = self.timeout {
            tokio_timeout(timeout, future()).await??
        } else {
            future().await?
        };

        // Read noise message from buffer
        let payload_len = match self.noise.read_message(&mut buffer[..len], payload_buffer) {
            Ok(HandshakeResult::InProgress(len)) => len,
            Ok(HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            }) => {
                self.split = Some((
                    Arc::new(RwLock::new(init)),
                    Arc::new(RwLock::new(resp)),
                    handshake_hash,
                ));
                len
            }
            Err(e) => Err(e)?,
        };
        Ok(payload_len)
    }

    //TODO: handle payload better, currently limitied to 1024 bytes (1 kb)
    async fn handle_handshake(&mut self) -> Result<(), TcpError> {
        let mut payload_buffer = [0u8; 1024];
        // Start with a read first if not initiator
        if !self.noise.is_initiator() {
            // Read and parse noise message
            let _ = self.read_message(&mut payload_buffer).await?;
        }

        // Take turns in handshake until complete
        while !self.noise.is_complete() {
            // Write noise message to buffer and send
            self.write_message(&mut []).await?;

            if !self.noise.is_complete() {
                // Read and parse noise message
                let _ = self.read_message(&mut payload_buffer).await?;
            }
        }
        Ok(())
    }

    pub async fn send_event<F: al_core::SerdeFormat, E: al_core::Event>(
        &self,
        event: E,
    ) -> Result<(), TcpError> {
        let format = F::default();
        let event: Box<dyn al_core::Event> = Box::new(event);

        // Serialize event to bytes
        let bytes = format
            .serialize_event(event.as_ref())
            .map_err(|e| TcpError::SerdeError(format.error_to_string(e)))?;

        // Send message
        Ok(self.send(bytes).await?)
    }

    pub async fn recv_event<F: al_core::SerdeFormat>(
        &self,
    ) -> Result<Box<dyn al_core::Event>, TcpError> {
        let format = F::default();
        let vec: Vec<u8> = self.recv().await?;
        format
            .deserialize_event_dyn(&vec)
            .map_err(|e| TcpError::SerdeError(format.error_to_string(e)))
    }

    async fn encrypt_bytes(
        &self,
        mut plaintext: Vec<u8>,
    ) -> Result<Vec<u8>, al_core::TransportError> {
        if let Some(split) = &self.split {
            let cipher = if self.noise.is_initiator() {
                &split.0
            } else {
                &split.1
            };

            let cipher = cipher.read().await;
            Ok(cipher
                .encrypt_with_ad(&[], plaintext.as_mut_slice())
                .map_err(|e| al_core::TransportError::Transport(format!("{:?}", e)))?)
        } else {
            Err(al_core::TransportError::Transport(
                TcpError::HandshakeIncomplete.to_string(),
            ))?
        }
    }

    async fn decrypt_bytes(
        &self,
        ciphertext_packet: &mut [u8],
    ) -> Result<Vec<u8>, al_core::TransportError> {
        if let Some(split) = &self.split {
            let cipher = if !self.noise.is_initiator() {
                &split.0
            } else {
                &split.1
            };

            let cipher = cipher.read().await;
            Ok(cipher
                .decrypt_with_ad(&[], ciphertext_packet)
                .map_err(|e| al_core::TransportError::Transport(format!("{:?}", e)))?)
        } else {
            Err(al_core::TransportError::Transport(
                TcpError::HandshakeIncomplete.to_string(),
            ))?
        }
    }
}

//TODO: maybe use `AsyncRuntime::spawn_blocking` instead of not supporting blocking?
/// Blocking operations are unsupported
impl<T: TransportItemRequirements, N: NonceTrait> Transport<T> for Tcp<N> {
    fn send_blocking(&self, _data: T) -> Result<(), al_core::TransportError> {
        Err(al_core::TransportError::UnSupported(
            "Tcp does not support blocking operations".to_string(),
        ))
    }

    fn send_batch_blocking(&self, _data: Vec<T>) -> Result<(), al_core::TransportError> {
        Err(al_core::TransportError::UnSupported(
            "Tcp does not support blocking operations".to_string(),
        ))
    }

    fn recv_blocking(&self) -> Result<T, al_core::TransportError> {
        Err(al_core::TransportError::UnSupported(
            "Tcp does not support blocking operations".to_string(),
        ))
    }

    fn recv_avaliable_blocking(&self) -> Result<Vec<T>, al_core::TransportError> {
        Err(al_core::TransportError::UnSupported(
            "Tcp does not support blocking operations".to_string(),
        ))
    }

    fn try_recv_blocking(&self) -> Result<Option<T>, al_core::TransportError> {
        Err(al_core::TransportError::UnSupported(
            "Tcp does not support blocking operations".to_string(),
        ))
    }

    fn send(
        &self,
        data: T,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), al_core::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            // Serialize T
            let bytes = bitcode::serialize(&data)
                .map_err(|e| al_core::TransportError::SerdeError(e.to_string()))?;

            // Encrypt using cipher
            let encrypted = self.encrypt_bytes(bytes).await?;

            // Send length prefixed encrypted message
            let mut stream = self.stream.lock().await;

            stream
                .write_all(&(encrypted.len() as u16).to_be_bytes())
                .await
                .map_err(|e| al_core::TransportError::Transport(e.to_string()))?;

            stream
                .write_all(&encrypted)
                .await
                .map_err(|e| al_core::TransportError::Transport(e.to_string()))?;

            Ok(())
        })
    }

    fn send_batch(
        &self,
        data: Vec<T>,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<(), al_core::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async move {
            for item in data {
                self.send(item).await?;
            }
            Ok(())
        })
    }

    fn recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<T, al_core::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            // Read length prefix
            let mut len_bytes = [0u8; 2];
            let mut stream = self.stream.lock().await;

            stream
                .read_exact(&mut len_bytes)
                .await
                .map_err(|e| al_core::TransportError::Transport(e.to_string()))?;

            let len = u16::from_be_bytes(len_bytes) as usize;

            if len > MAX_MSG_BYTE_LEN {
                return Err(al_core::TransportError::Transport(format!(
                    "Message size {} exceeds the max {}",
                    len, MAX_MSG_BYTE_LEN
                )));
            }

            let mut buffer = self.buffer.lock().await;
            // Read encrypted message
            stream
                .read_exact(&mut buffer[..len])
                .await
                .map_err(|e| al_core::TransportError::Transport(e.to_string()))?;

            // Release stream after data is copied to buffer
            drop(stream);

            // Decrypt bytes
            let mut decrypted = self.decrypt_bytes(&mut buffer[..len]).await?;

            // Release buffer after data is used
            drop(buffer);

            // Deserialize to T
            let t = bitcode::deserialize::<T>(&decrypted)
                .map_err(|e| al_core::TransportError::SerdeError(e.to_string()));

            // zeroize the decrypted bytes
            decrypted.zeroize();

            // Return the owned T
            Ok(t?)
        })
    }

    fn recv_avaliable(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Vec<T>, al_core::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            let mut result = Vec::new();
            loop {
                match tokio_timeout(std::time::Duration::from_millis(1), self.recv()).await {
                    Ok(Ok(item)) => result.push(item),
                    Ok(Err(e)) => Err(e)?,
                    Err(_) => break,
                }
            }
            Ok(result)
        })
    }

    fn try_recv(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::prelude::rust_2024::Future<Output = Result<Option<T>, al_core::TransportError>>
                + Send
                + Sync
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(
                match tokio_timeout(std::time::Duration::from_millis(1), self.recv()).await {
                    Ok(Ok(item)) => Some(item),
                    Ok(Err(e)) => Err(e)?,
                    Err(_) => None,
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{CommandDispatcher, ConnectionManager, Tcp};
    use al_core::{Event, Transport};
    use al_crypto::Monotonic;
    use std::{sync::Arc, time::Duration};
    use tokio::sync::RwLock;

    const _TEST_MSG: &str = "secret message";
    const LOCAL_ADDR: &str = "127.0.0.1:7878";
    const TEST_PATTERN: crate::HandshakePattern = crate::HandshakePattern::NN;
    const TEST_PROLOGUE: &str = "";
    const _TEST_STATIC_I: [u8; 32] = [
        54, 204, 226, 149, 59, 170, 202, 179, 39, 51, 78, 144, 190, 98, 38, 222, 177, 244, 71, 48,
        232, 63, 157, 99, 137, 117, 121, 51, 144, 223, 137, 130,
    ];
    const _TEST_STATIC_I_PUB: [u8; 32] = [
        167, 1, 217, 60, 243, 178, 129, 109, 174, 99, 120, 54, 173, 205, 101, 4, 84, 98, 199, 118,
        153, 184, 85, 95, 179, 160, 172, 182, 33, 122, 100, 122,
    ];
    const _TEST_STATIC_R: [u8; 32] = [
        152, 120, 43, 42, 37, 109, 46, 119, 178, 204, 89, 29, 45, 109, 174, 126, 253, 212, 208,
        237, 90, 127, 112, 2, 195, 224, 225, 151, 222, 97, 118, 154,
    ];
    const _TEST_STATIC_R_PUB: [u8; 32] = [
        48, 203, 114, 127, 182, 56, 24, 179, 60, 87, 240, 145, 136, 107, 230, 212, 151, 154, 4,
        235, 104, 35, 131, 93, 247, 14, 98, 74, 152, 206, 183, 1,
    ];

    #[al_core::event]
    struct TestEventA(u8);

    #[al_core::event]
    struct TestEventB(u8);

    #[tokio::test]
    async fn nn_client_server_echo() {
        // Register event types
        al_core::register_event!(TestEventA);
        al_core::register_event!(TestEventB);

        // Create dispatcher
        let mut dispatcher = CommandDispatcher::new();

        // Register two event handlers for `TestEventA`
        dispatcher
            .register_event::<TestEventA, _>(|_, _, _| async { println!("Received TestEventA") })
            .await;
        dispatcher
            .register_event::<TestEventA, _>(|conn_id, connections, event| async move {
                let _x = connections.get(0).await;
                println!("Received TestEventA from conn {}: {:?}", conn_id, event)
            })
            .await;

        // Register two event handlers for `TestEventB`
        dispatcher
            .register_event(|_, _, _: TestEventB| async { println!("Received TestEventB") })
            .await;
        dispatcher
            .register_event(|conn_id, connections, event: TestEventB| async move {
                connections.get(conn_id).await;
                println!("Received TestEventB: {:?}", event)
            })
            .await;

        // Register catch-all command handler
        dispatcher
            .register_command(|conn_id, _, cmd| async move {
                match cmd.event_type_name() {
                    Some(type_name) => println!(
                        "\nCommand from conn {} is event type {}: {:?}",
                        conn_id, type_name, cmd
                    ),
                    None => println!("Command from conn {} is command: {:?}", conn_id, cmd),
                }
            })
            .await;

        // Start a server on another thread
        let token = Arc::new(RwLock::new(false));
        let token_clone = token.clone();
        let mut dispatcher_clone = dispatcher.deep_clone().await;

        let server_handle = tokio::spawn(async move {
            // Echo commands back to sender
            dispatcher_clone
                .register_command(|id, connections, cmd| async move {
                    if let Some(conn) = connections.get(id).await {
                        let _ = conn.send(cmd).await;
                    }
                })
                .await;
            let connection_manager = Arc::new(ConnectionManager::new());
            Tcp::<Monotonic>::run_server_with_shutdown(
                LOCAL_ADDR,
                Some(Duration::from_millis(10)),
                TEST_PATTERN,
                TEST_PROLOGUE.as_bytes().to_vec(),
                None,
                None,
                |tcp| {
                    let dispatcher = dispatcher_clone.clone();
                    let connection_manager_clone = connection_manager.clone();
                    tokio::spawn(async move {
                        let tcp = Arc::new(tcp);
                        let conn_id = connection_manager_clone.insert(tcp.clone()).await;

                        loop {
                            match tcp.recv().await {
                                Ok(cmd) => {
                                    dispatcher
                                        .dispatch(conn_id, connection_manager_clone.clone(), cmd)
                                        .await
                                }
                                Err(e) => eprintln!("Error: {:?}", e),
                            }
                        }
                    });
                },
                token_clone,
            )
            .await
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect to the server
        let connection_manager = Arc::new(ConnectionManager::new());
        let tcp = Arc::new(
            Tcp::<Monotonic>::connect(
                LOCAL_ADDR,
                None,
                TEST_PATTERN,
                TEST_PROLOGUE.as_bytes().to_vec(),
                None,
                None,
            )
            .await
            .unwrap(),
        );
        let conn_id = connection_manager.insert(tcp.clone()).await;

        // Send command to server
        tcp.send(al_core::Command::Pulse).await.unwrap();

        // Recv echo back from server
        let cmd: al_core::Command = tcp.recv().await.unwrap();
        assert_eq!(al_core::Command::Pulse, cmd);
        dispatcher.dispatch(conn_id, connection_manager.clone(), cmd).await;

        // Send event as command to server
        tcp.send(TestEventA(0).to_cmd()).await.unwrap();

        // Recv echo back from server
        let cmd: al_core::Command = tcp.recv().await.unwrap();
        assert_eq!(TestEventA(0), cmd.downcast_event().unwrap());
        dispatcher.dispatch(conn_id, connection_manager.clone(), cmd).await;

        // Send event as command to server
        tcp.send(TestEventA(5).to_cmd()).await.unwrap();

        // Recv echo back from server
        let cmd: al_core::Command = tcp.recv().await.unwrap();
        assert_eq!(TestEventA(5), cmd.downcast_event().unwrap());
        dispatcher.dispatch(conn_id, connection_manager.clone(), cmd).await;

        // Send event as command to server
        tcp.send(TestEventB(10).to_cmd()).await.unwrap();

        // Recv echo back from server
        let cmd: al_core::Command = tcp.recv().await.unwrap();
        assert_eq!(TestEventB(10), cmd.downcast_event().unwrap());
        dispatcher.dispatch(conn_id, connection_manager.clone(), cmd).await;

        // Send event as command to server
        tcp.send(TestEventB(15).to_cmd()).await.unwrap();

        // Recv echo back from server
        let cmd: al_core::Command = tcp.recv().await.unwrap();
        assert_eq!(TestEventB(15), cmd.downcast_event().unwrap());
        dispatcher.dispatch(conn_id, connection_manager.clone(), cmd).await;

        // Stop server and await handle
        *token.write().await = true;
        server_handle.await.unwrap().unwrap();
    }
}
