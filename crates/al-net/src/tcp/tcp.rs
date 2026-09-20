use crate::tcp::{TcpError, TcpSocket};
use al_crypto::NonceTrait;
use al_events::DynMessage;
use al_secure::noise::{
    cipher_state::CipherState,
    handshake_pattern::HandshakePattern,
    handshake_state::{HandshakeResult, HandshakeState},
    KeyPair, Noise, PublicKey, HASHLEN, MAX_MSG_BYTE_LEN,
};
use al_structures::{cancellation::CancellationToken, serde_utils::serde_registries::FormatId};
use al_transport::{
    transports::BoundaryQueue, Action, Backpressure, Transport, TransportError, Vacancy,
};
use std::{sync::Arc, task::Poll};
use tokio::task::JoinSet;
use zeroize::Zeroize;

pub async fn run_server_with_shutdown<N: NonceTrait, F: Fn(Tcp<N>)>(
    listener: tokio::net::TcpListener,
    pattern: HandshakePattern,
    prologue: Vec<u8>,
    local_static: Option<KeyPair>,
    remote_static: Option<PublicKey>,
    token: CancellationToken,
    on_connection: F,
) -> std::io::Result<()> {
    let mut tasks = JoinSet::<()>::new();

    loop {
        tokio::select! {
            biased;
            _ = token.cancelled() => break,
            accept = listener.accept() => {
                let (stream, _peer) = match accept {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("Tcp server: Accept error: {e}");
                        continue;
                    }
                };
                let (tcp, socket) = match Tcp::<N>::new_responder(
                    pattern.clone(),
                    prologue.clone(),
                    local_static.clone(),
                    remote_static.clone()
                ) {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("Tcp server: Noise error: {e}");
                        continue;
                    }
                };
                on_connection(tcp);

                let conn_token = token.clone();
                tasks.spawn(async move {
                    let _ = socket.run_with_cancel(stream, conn_token).await;
                });
            }
        }
    }

    // Attempt to join gracefully
    if tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while tasks.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        tasks.shutdown().await;
    }
    Ok(())
}

const MESSAGE_CAPACITY: usize = 1024;

pub struct Tcp<N: NonceTrait> {
    // Noise state
    noise: HandshakeState<N>,
    split: Option<(
        CipherState<N>, // Initiator sends with first
        CipherState<N>, // Responder sends with second
        [u8; HASHLEN],
    )>,
    handshake_waiting_read: bool,
    handshake_payload: Option<Vec<u8>>,

    //REVIEW: Use two buffers? the current is renamed `box_buffer`
    // and a smaller stack buffer (eg [u8; 1024]) could be used when possible?
    buffer: Box<[u8; MAX_MSG_BYTE_LEN]>,

    // Data recieved from the transport before handshake is complete
    pre_buffer: std::collections::VecDeque<Vec<u8>>,
    // Data recieved from the wire.
    ready: std::collections::VecDeque<Vec<u8>>,

    // Ciphertext to be written to the socket
    to_wire: Arc<BoundaryQueue<Vec<u8>>>,
    // Ciphertext read from the socket
    from_wire: Arc<BoundaryQueue<Vec<u8>>>,

    // from_wire framing state
    pending_len: Option<u16>,
    pending_bytes: std::collections::VecDeque<u8>,

    error: Option<TcpError>,
}

impl<N: NonceTrait> std::fmt::Debug for Tcp<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tcp")
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
            .field("buffer", &self.buffer.len())
            .field("pending_len", &self.pending_len)
            .field("pending_bytes", &self.pending_bytes)
            .field("pre-buffer", &self.pre_buffer.len())
            .field("ready", &self.ready.len())
            //REVIEW: include the len of the two boundary queues?
            .field("error", &self.error)
            .finish()
    }
}

impl<N: NonceTrait> Tcp<N> {
    // Client
    pub fn new_initiator(
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
    ) -> Result<(Self, TcpSocket), TcpError> {
        let noise = Noise::new(pattern)
            .local_static(local_static)
            .remote_static(remote_static)
            .with_prologue(prologue)
            .initiate()?;
        let (mut tcp, socket) = Self::new(noise);
        tcp.maybe_produce_handshake()?;
        Ok((tcp, socket))
    }

    // Server
    pub fn new_responder(
        pattern: HandshakePattern,
        prologue: Vec<u8>,
        local_static: Option<KeyPair>,
        remote_static: Option<PublicKey>,
    ) -> Result<(Self, TcpSocket), TcpError> {
        let noise = Noise::new(pattern)
            .local_static(local_static)
            .remote_static(remote_static)
            .with_prologue(prologue)
            .respond()?;
        let (mut tcp, socket) = Self::new(noise);
        tcp.handshake_waiting_read = true;
        Ok((tcp, socket))
    }

    fn new(noise: HandshakeState<N>) -> (Self, TcpSocket) {
        let to_wire = BoundaryQueue::<Vec<u8>>::new();
        let from_wire = BoundaryQueue::<Vec<u8>>::new();
        let tcp = Self {
            noise,
            split: None,
            handshake_waiting_read: false,
            handshake_payload: None,
            to_wire: to_wire.clone(),
            from_wire: from_wire.clone(),
            pending_bytes: std::collections::VecDeque::new(),
            pending_len: None,
            buffer: Box::new([0u8; MAX_MSG_BYTE_LEN]),
            pre_buffer: std::collections::VecDeque::new(),
            ready: std::collections::VecDeque::new(),
            error: None,
        };
        (tcp, TcpSocket::new(to_wire, from_wire))
    }

    /*pub async fn connect<A: ToSocketAddrs>(
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

        let (reader, writer) = stream.into_split();
        let mut tcp = Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            buffer: Mutex::new(Box::new([0u8; MAX_MSG_BYTE_LEN])),
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

        let (reader, writer) = stream.into_split();

        let mut tcp = Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            buffer: Mutex::new(Box::new([0u8; MAX_MSG_BYTE_LEN])),
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
                //REVIEW: Could I use `tokio::task::yield_now().await` instead?
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
        token: Arc<RwLock<bool>>,
        f: F,
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
                //REVIEW: Could I use `tokio::task::yield_now().await` instead?
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
    }*/

    /*//REVIEW: Could I pre-handle handshake?
    //REVIEW: handle payload better, currently limitied to 1024 bytes (1 kb)
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
    }*/

    fn encrypt_and_send(&mut self, mut data: Vec<u8>) -> Result<(), TcpError> {
        let split = self.split.as_ref().ok_or(TcpError::HandshakeIncomplete)?;
        let cipher = if self.noise.is_initiator() {
            &split.0
        } else {
            &split.1
        };
        let ciphertext = cipher.encrypt_with_ad(&[], data.as_mut_slice())?;

        let mut packet = Vec::with_capacity(2 + ciphertext.len());
        packet.extend_from_slice(&(ciphertext.len() as u16).to_be_bytes());
        packet.extend_from_slice(&ciphertext);

        self.to_wire
            .send(packet)
            .map_err(|_| TcpError::WireQueueClosed)?;
        Ok(())
    }

    fn process_wire_bytes(&mut self, bytes: &[u8]) -> Result<(), TcpError> {
        self.pending_bytes.extend(bytes);
        loop {
            if self.pending_len.is_none() {
                if self.pending_bytes.len() < 2 {
                    return Ok(());
                }
                let hi = self.pending_bytes.pop_front().unwrap();
                let lo = self.pending_bytes.pop_front().unwrap();
                let len = u16::from_be_bytes([hi, lo]) as usize;
                if len > MAX_MSG_BYTE_LEN {
                    return Err(TcpError::MessageTooLarge(len, MAX_MSG_BYTE_LEN));
                }
                self.pending_len = Some(len as u16);
            }

            let len = self.pending_len.unwrap() as usize;
            if self.pending_bytes.len() < len {
                return Ok(());
            }
            let mut frame: Vec<u8> = self.pending_bytes.drain(..len).collect();
            self.pending_len = None;

            if self.noise.is_complete() {
                let split = self.split.as_ref().ok_or(TcpError::HandshakeIncomplete)?;
                let cipher = if self.noise.is_initiator() {
                    &split.1
                } else {
                    &split.0
                };
                let plaintext = cipher.decrypt_with_ad(&[], frame.as_mut_slice())?;
                self.ready.push_back(plaintext);
            } else {
                let mut payload = self.handshake_payload.take().unwrap_or_default();
                let result = self.noise.read_message(&mut frame, &mut payload)?;
                match result {
                    HandshakeResult::InProgress(_) => {}
                    HandshakeResult::Complete {
                        init,
                        resp,
                        handshake_hash,
                        ..
                    } => {
                        self.split = Some((init, resp, handshake_hash));
                    }
                }
                self.handshake_waiting_read = false;
                self.maybe_produce_handshake()?;
                self.flush_pending_app_data()?;
            }
        }
    }

    fn maybe_produce_handshake(&mut self) -> Result<(), TcpError> {
        if self.noise.is_complete() || self.handshake_waiting_read {
            return Ok(());
        }
        let result = self
            .noise
            .write_message(&mut [], self.buffer.as_mut_slice())?;
        let len = match result {
            HandshakeResult::InProgress(len) => len,
            HandshakeResult::Complete {
                init,
                resp,
                handshake_hash,
                len,
            } => {
                self.split = Some((init, resp, handshake_hash));
                len
            }
        };
        let mut packet = Vec::with_capacity(2 + len as usize);
        packet.extend_from_slice(&len.to_be_bytes());
        packet.extend_from_slice(&self.buffer[..len as usize]);
        self.to_wire
            .send(packet)
            .map_err(|_| TcpError::WireQueueClosed)?;
        self.buffer.zeroize();
        self.handshake_waiting_read = true;
        Ok(())
    }

    fn flush_pending_app_data(&mut self) -> Result<(), TcpError> {
        if !self.noise.is_complete() {
            return Ok(());
        }
        while let Some(data) = self.pre_buffer.pop_front() {
            self.encrypt_and_send(data)?;
        }
        Ok(())
    }

    pub fn send_message(
        &mut self,
        format_id: FormatId,
        message: &DynMessage,
    ) -> Result<(), TcpError> {
        let mut bytes = Vec::new();
        message.to_format(format_id, &mut bytes)?;
        self.handle_incoming(bytes).map_err(TcpError::Backpressure)
    }

    pub fn recv_message(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Action<DynMessage>> {
        self.poll_action(cx).map(|action| match action {
            Action::Data(data) => match DynMessage::from_format_slice(&data) {
                Ok(msg) => Action::Data(msg),
                Err(e) => Action::Error(TcpError::MessageError(e).into()),
            },
            Action::Pending => Action::Pending,
            Action::Error(e) => Action::Error(e),
        })
    }
}

impl From<TcpError> for TransportError {
    fn from(value: TcpError) -> Self {
        if let TcpError::DriverError(al_transport::DriverError::Transport(err)) = value {
            err
        } else {
            TransportError::Custom(Box::new(value))
        }
    }
}

impl<N: NonceTrait> Transport<Vec<u8>> for Tcp<N> {
    fn handle_incoming(&mut self, data: Vec<u8>) -> Result<(), Backpressure> {
        if self.error.is_some() {
            return Err(Backpressure::Closed);
        }
        if !self.noise.is_complete() {
            if self.pre_buffer.len() >= MESSAGE_CAPACITY {
                return Err(Backpressure::BufferFull);
            }
            self.pre_buffer.push_back(data);
            return Ok(());
        }
        self.encrypt_and_send(data).map_err(|e| {
            self.error = Some(e);
            Backpressure::Closed
        })
    }

    fn poll_action(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Action<Vec<u8>>> {
        if let Some(e) = self.error.take() {
            return Poll::Ready(Action::Error(e.into()));
        }

        // Drain wire bytes until the queue is empty, then register the waker
        loop {
            match self.from_wire.poll_recv(cx) {
                Poll::Ready(Some(bytes)) => {
                    if let Err(e) = self.process_wire_bytes(&bytes) {
                        return Poll::Ready(Action::Error(e.into()));
                    }
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Action::Error(TransportError::Backpressure(
                        Backpressure::Closed,
                    )))
                }
                Poll::Pending => break,
            }
        }

        if let Some(data) = self.ready.pop_front() {
            if !self.ready.is_empty() {
                cx.waker().wake_by_ref();
            }
            return Poll::Ready(Action::Data(data));
        }
        Poll::Pending
    }

    fn has_space(&self) -> al_transport::Vacancy {
        if self.error.is_some() {
            Vacancy::Backpressure(Backpressure::Closed)
        } else {
            let vacancy = MESSAGE_CAPACITY.saturating_sub(self.pre_buffer.len());
            if vacancy > 0 {
                Vacancy::Vacancy(vacancy)
            } else {
                Vacancy::Backpressure(Backpressure::BufferFull)
            }
        }
    }

    fn status(&self) -> String {
        format!(
            "Tcp {{ handshake complete: {}, initiator: {}, pending_bytes: {}, buffer: {}, ready: {}, error: {:?} }}",
            self.noise.is_complete(),
            self.noise.is_initiator(),
            self.pending_bytes.len(),
            self.pre_buffer.len(),
            self.ready.len(),
            self.error
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::tcp::{run_server_with_shutdown, Tcp};
    use al_crypto::{Monotonic, NonceTrait};
    use al_events::{
        command, event, register_command, register_event, register_format, CommandHelpers,
        EventHelpers,
    };
    use al_secure::noise::handshake_pattern::HandshakePattern;
    use al_structures::cancellation::CancellationToken;
    use al_transport::{Action, Transport};
    use std::time::Duration;

    const _TEST_MSG: &str = "secret message";
    const LOCAL_ADDR: &str = "127.0.0.1:7878";
    const TEST_PATTERN: HandshakePattern = HandshakePattern::NN;
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

    #[event]
    struct TestEventA(u8);

    #[event]
    struct TestEventB(u8);

    #[command]
    struct Pulse;

    async fn echo_driver<N: NonceTrait>(mut tcp: Tcp<N>, token: CancellationToken) {
        loop {
            if !tcp.has_space().is_available() {
                continue;
            }
            let action = tokio::select! {
                biased;
                _ = token.cancelled() => break,
                action = std::future::poll_fn(|cx| tcp.poll_action(cx)) => action,
            };

            match action {
                Action::Data(data) => {
                    if tcp.handle_incoming(data).is_err() {
                        break;
                    }
                }
                Action::Error(e) => {
                    eprintln!("echo driver error: {e}");
                    break;
                }
                Action::Pending => {}
            }
        }
    }

    #[tokio::test]
    async fn nn_client_server_echo() {
        let format_id = register_format!(al_structures::serde_utils::formats::JsonFormat).unwrap();
        //TODO: get ids from register macros like format
        // Register event types
        register_event!(TestEventA);
        register_event!(TestEventB);
        register_command!(Pulse);

        let server_token = CancellationToken::new();
        let client_token = CancellationToken::new();

        // ----- Server -----
        let server_handle = {
            let accept_token = server_token.clone();
            let conn_token = server_token.clone();
            tokio::spawn(async move {
                let listener = tokio::net::TcpListener::bind(LOCAL_ADDR).await.unwrap();
                run_server_with_shutdown::<Monotonic, _>(
                    listener,
                    TEST_PATTERN,
                    TEST_PROLOGUE.as_bytes().to_vec(),
                    None,
                    None,
                    accept_token,
                    move |tcp| {
                        let token = conn_token.clone();
                        tokio::spawn(async move {
                            echo_driver(tcp, token).await;
                        });
                    },
                )
                .await
                .unwrap();
            })
        };

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // ----- Client -----
        let (mut tcp, socket) = Tcp::<Monotonic>::new_initiator(
            TEST_PATTERN,
            TEST_PROLOGUE.as_bytes().to_vec(),
            None,
            None,
        )
        .unwrap();

        let socket_handle = {
            let token = client_token.clone();
            tokio::spawn(
                async move { socket.connect_with_cancel(LOCAL_ADDR, token).await.unwrap() },
            )
        };

        // ----- Msg Queue -----
        let msgs = [
            Pulse.to_msg(),
            TestEventA(1).to_msg(),
            TestEventB(42).to_msg(),
        ];
        tcp.handle_incoming(b"hello world".to_vec()).unwrap();
        for msg in &msgs {
            tcp.send_message(format_id, msg).unwrap();
        }

        // Wait for echo
        let echoed = std::future::poll_fn(|cx| tcp.poll_action(cx)).await;
        match echoed {
            Action::Data(data) => assert_eq!(data, b"hello world"),
            Action::Error(e) => panic!("transport error: {e}"),
            Action::Pending => panic!("poll_action returned `Pending` without a matching wake"),
        }
        for msg in msgs {
            let echoed = std::future::poll_fn(|cx| tcp.recv_message(cx)).await;
            match echoed {
                Action::Data(data) => {
                    assert_eq!(data, msg)
                }
                Action::Error(e) => panic!("transport error: {e}"),
                Action::Pending => panic!("poll_action returned `Pending` without a matching wake"),
            }
        }

        // ----- Cleanup -----
        server_token.cancel();
        client_token.cancel();
        server_handle.await.unwrap();
        socket_handle.await.unwrap();
    }
}
