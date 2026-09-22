use al_events::{register_event, register_format, DynMessage, EventHelpers, FormatId};
use al_net::{
    message_dispatcher::{DynMessageDispatcher, DynMessageKey},
    tcp::{Tcp, TcpError, TcpSocket},
    TokioWaker,
};
use al_structures::{
    cancellation::CancellationToken,
    collections::storage::{
        utils::{indexed::IndexedStorage, keyed::KeyedStorage},
        CowStorage,
    },
};
use al_transport::{
    dispatcher::Handler,
    splice::{splice_async, SpliceHandle},
    transports::{BoundaryQueue, FallibleSink},
    Driver, DriverError, TransportError, TransportId,
};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::{JoinHandle, JoinSet},
};

pub type ConnId = u64;

pub type Payload = Vec<u8>;
pub type TaggedBytes = (ConnId, Payload);
pub type TaggedMsg = (ConnId, DynMessage);

pub type MessageHandler = Handler<DynMessage, AppContext, AppStateRef>;
pub type AnyStore = CowStorage<Vec<MessageHandler>>;
pub type KeyedStore = CowStorage<HashMap<DynMessageKey, HandlerStore>>;
pub type HandlerStore = Vec<MessageHandler>;

pub type AppDispatcher =
    DynMessageDispatcher<AppContext, AppStateRef, AnyStore, KeyedStore, HandlerStore>;

pub type TransportIdStorage = HashMap<ConnId, TransportId>;
pub type ConnIdStorage = HashMap<TransportId, ConnId>;
pub type LiveStorage = Vec<ConnId>;
pub type NetTask = NetworkTask<al_crypto::Monotonic, TransportIdStorage, ConnIdStorage, LiveStorage>;

#[tokio::main]
async fn main() {
    register_event!(NetEvent);
    register_event!(Broadcast);
    register_event!(Msg);
    let format_id = register_format!(al_structures::serde_utils::formats::JsonFormat).unwrap();

    let tui_handle = tokio::task::spawn_blocking(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let app = {
                let (width, height) = crossterm::terminal::size().unwrap();
                App::new(
                    width,
                    height,
                    format_id,
                    AnyStore::default(),
                    KeyedStore::default(),
                )
                .await
                .unwrap()
            };
            if let Err(e) = App::run_tui(app).await {
                eprintln!("TUI error: {:?}", e);
            }
        })
    });

    // Wait for tui thread
    if let Err(e) = tui_handle.await {
        eprintln!("TUI thread error: {}", e);
    }
}

#[al_events::event]
pub enum NetEvent {
    #[default]
    NoOp,
    Connected(ConnId),
    Disconnected(ConnId),
}

#[al_events::event]
pub struct Broadcast {
    except: Option<Vec<ConnId>>,
    msg: Vec<u8>,
}

impl Broadcast {
    pub fn new(except: Option<Vec<ConnId>>, msg: Vec<u8>) -> Self {
        Self { except, msg }
    }
}

#[al_events::event]
pub struct Msg {
    username: Vec<u8>,
    msg: Vec<u8>,
}

impl Msg {
    pub fn new(username: impl Into<Vec<u8>>, msg: impl Into<Vec<u8>>) -> Self {
        Self {
            username: username.into(),
            msg: msg.into(),
        }
    }

    pub fn to_chat(&self) -> Result<String, std::str::Utf8Error> {
        match str::from_utf8(&self.username) {
            Ok(name) => match str::from_utf8(&self.msg) {
                Ok(msg) => Ok(format!("{}: {}", name, msg)),
                Err(e) => Err(e)?,
            },
            Err(e) => Err(e)?,
        }
    }
}

pub type AppStateRef = Arc<RwLock<AppState>>;
pub struct AppState {
    history: VecDeque<Msg>,
    max_history_len: usize,
    /// The single connection this app owns (client mode only).
    local_conn: Option<ConnId>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(150),
            max_history_len: 100,
            local_conn: None,
        }
    }
}

#[derive(Clone)]
pub struct AppContext {
    /// `Some(conn_id)` if this message came from a peer, `None` if local.
    source: Option<ConnId>,
    net: Arc<NetworkHandle>,
}

impl AppContext {
    pub fn new(source: Option<ConnId>, net: Arc<NetworkHandle>) -> Self {
        Self { source, net }
    }
}

pub struct App {
    // Terminal
    dims: (u16, u16),
    stop: bool,
    input: String,
    cursor_pos: u16,
    username: String,
    // App
    state: AppStateRef,
    net: Arc<NetworkHandle>,
    dispatcher: Arc<AppDispatcher>,
}

impl App {
    pub async fn new(
        width: u16,
        height: u16,
        format_id: FormatId,
        any: AnyStore,
        keyed: KeyedStore,
    ) -> Result<Arc<RwLock<Self>>, TuiError> {
        let state = Arc::new(RwLock::new(AppState::new()));
        let handle = NetworkHandle::new(format_id).await;
        let handle = Arc::new(handle);

        let dispatcher = Arc::new(DynMessageDispatcher::new(state.clone(), any, keyed));

        dispatcher
            .register_event(|_, state, msg: Msg| {
                Box::pin(async move {
                    let mut s = state.write().await;
                    s.history.push_back(msg);
                    if s.history.len() > s.max_history_len {
                        s.history.pop_front();
                    }
                })
            })
            .await?;

        dispatcher
            .register_event(|ctx, _, msg: Msg| {
                let net = ctx.net.clone();
                let source = ctx.source;
                Box::pin(async move {
                    if !net.is_server() {
                        return;
                    }
                    let Some(src) = source else {
                        return;
                    };
                    if let Err(e) = net.broadcast(Some(vec![src]), msg.to_msg()) {
                        eprintln!("Server failed to broadcast message: {e}");
                    }
                })
            })
            .await?;

        dispatcher
            .register_event(|ctx, state, evt: NetEvent| {
                Box::pin(async move {
                    match evt {
                        NetEvent::NoOp => {}
                        NetEvent::Connected(conn_id) => {
                            if ctx.net.is_server() {
                                let history: Vec<Msg> =
                                    { state.read().await.history.iter().cloned().collect() };
                                for msg in history {
                                    if let Err(e) = ctx.net.send_to(conn_id, msg.to_msg()) {
                                        eprintln!("Server failed to send history message: {e}")
                                    }
                                }
                            } else {
                                state.write().await.local_conn = Some(conn_id);
                            }
                        }
                        NetEvent::Disconnected(conn_id) => {
                            let mut s = state.write().await;
                            s.history.push_back(Msg::new(
                                "Server",
                                format!("Client {conn_id} disconnected"),
                            ));
                            if s.history.len() > s.max_history_len {
                                s.history.pop_front();
                            }
                            if s.local_conn == Some(conn_id) {
                                s.local_conn = None;
                            }
                        }
                    }
                })
            })
            .await?;

        let app = Arc::new(RwLock::new(App {
            dims: (width, height),
            stop: false,
            input: String::new(),
            cursor_pos: 0,
            username: String::new(),
            state,
            net: handle,
            dispatcher,
        }));

        Ok(app)
    }

    pub async fn run_tui(state: Arc<RwLock<Self>>) -> Result<(), TuiError> {
        // ---- Terminal setup ----
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

        // ---- Prompts ----
        {
            let mut s = state.write().await;
            s.get_user_name(&mut stdout)?;
            s.start_client_or_server(&mut stdout).await?;
        }

        let (net, dispatcher) = {
            let s = state.read().await;
            (s.net.clone(), s.dispatcher.clone())
        };

        // Start with non-empty last_input to trigger first draw
        let mut last_input = ".".to_string();
        let mut slice_0 = vec![Msg::new(Vec::new(), Vec::new()); 150];
        let mut slice_1 = vec![Msg::new(Vec::new(), Vec::new()); 150];
        let mut last_history = (slice_0.as_mut_slice(), slice_1.as_mut_slice());

        // ---- Main loop ----
        loop {
            if state.read().await.stop {
                break;
            }

            // 1. Connection lifecycle events.
            for evt in net.events.recv_available().unwrap_or_default() {
                dispatcher
                    .dispatch(AppContext::new(None, net.clone()), evt.to_msg())
                    .await?;
            }

            // 2. Inbound messages from peers.
            for (conn_id, msg) in net.app_in.recv_available().unwrap_or_default() {
                dispatcher
                    .dispatch(AppContext::new(Some(conn_id), net.clone()), msg)
                    .await?;
            }

            // 3. Terminal input.
            if crossterm::event::poll(Duration::from_millis(20))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.kind != crossterm::event::KeyEventKind::Release {
                        state.write().await.handle_key(key).await?;
                    }
                }
            }

            // 4. Draw.
            state
                .read()
                .await
                .draw(&mut last_history, &mut last_input)
                .await?;
        }

        // ---- Shutdown ----
        disable_raw_mode()?;
        execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;

        net.token.cancel();
        if let Some(h) = state.write().await.net.handle.lock().await.take() {
            let _ = h.await;
        }
        Ok(())
    }

    async fn start_client_or_server(
        &mut self,
        stdout: &mut std::io::Stdout,
    ) -> Result<(), TuiError> {
        self.get_client_server(stdout)?;
        let addr = self.get_address(stdout)?;

        let task = self
            .net
            .task
            .lock()
            .await
            .take()
            .ok_or_else(|| TuiError::AppError("Net task already taken.".to_string()))?;

        if self.net.is_server() {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            self.net
                .handle
                .lock()
                .await
                .replace(tokio::spawn(async move { task.run(Some(listener)).await }));
        } else {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            let (tcp, socket) = Tcp::<al_crypto::Monotonic>::new_initiator(
                al_secure::noise::handshake_pattern::HandshakePattern::NN,
                Vec::new(),
                None,
                None,
            )?;
            let mut task = task;
            task.add_tcp(tcp, socket, stream).await?;
            self.net
                .handle
                .lock()
                .await
                .replace(tokio::spawn(async move { task.run(None).await }));
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<(), std::io::Error> {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
                self.stop = true;
            }

            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos as usize, c);
                self.cursor_pos += 1;
            }
            KeyCode::Backspace if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.input.remove(self.cursor_pos as usize);
            }
            KeyCode::Left => self.cursor_pos = self.cursor_pos.saturating_sub(1),
            KeyCode::Right if (self.cursor_pos as usize) < self.input.len() => {
                self.cursor_pos += 1;
            }

            KeyCode::Enter => {
                if self.input.trim().is_empty() {
                    return Ok(());
                }
                let text = self.input.trim().to_owned();
                let msg = Msg::new(self.username.as_bytes(), text);

                // get destination from state.
                let (target, is_server) = {
                    let s = self.state.read().await;
                    (s.local_conn, self.net.is_server())
                };

                // Send: server broadcasts, client sends to its single conn.
                if is_server {
                    let _ = self.net.broadcast(None, msg.clone().to_msg());
                } else if let Some(id) = target {
                    let _ = self.net.send_to(id, msg.clone().to_msg());
                }

                // add to own history.
                let mut s = self.state.write().await;
                s.history.push_back(msg);
                if s.history.len() > s.max_history_len {
                    s.history.pop_front();
                }
                drop(s);

                self.input.clear();
                self.cursor_pos = 0;
            }

            KeyCode::Esc => self.stop = true,
            _ => {}
        }
        Ok(())
    }

    fn get_user_name(&mut self, stdout: &mut std::io::Stdout) -> Result<(), TuiError> {
        let title =
            "Chat Room === Choose Username === Max 10 characters, UTF8 only === Ctrl+Q to quit";
        let prompt = "Enter username (max 10 chars, UTF8 only): ";
        let error_msg = "=== Invalid Username! ===";
        let mut entered_invalid = false;
        self.username = loop {
            let input = get_input_with_display(
                stdout,
                self.dims.0,
                title,
                prompt,
                Some(10),
                error_msg,
                entered_invalid,
            )?;

            if input.is_empty() {
                entered_invalid = true;
                continue;
            }

            break input;
        };
        Ok(())
    }

    fn get_client_server(&mut self, stdout: &mut std::io::Stdout) -> Result<(), TuiError> {
        let title = "Chat Room === Choose client or server === Ctrl+Q to quit";
        let prompt = "Start as (c)lient or (s)erver? [s/c]: ";
        let error_msg = "=== Invalid Choice! ===";
        let mut entered_invalid = false;
        if loop {
            let input = get_input_with_display(
                stdout,
                self.dims.0,
                title,
                prompt,
                Some(1),
                error_msg,
                entered_invalid,
            )?;

            match input.as_str() {
                "s" => break true,
                "c" => break false,
                _ => {
                    entered_invalid = true;
                    continue;
                }
            }
        } {
            self.net.mark_server();
        }
        Ok(())
    }

    fn get_address(&mut self, stdout: &mut std::io::Stdout) -> Result<String, TuiError> {
        let title = "Chat Room === Enter address === Ctrl+Q to quit";
        let prompt = if self.net.is_server() {
            "Enter port to listen on [default: 7878]: "
        } else {
            "Enter address to connect to [default: 127.0.0.1:7878]: "
        };
        let error_msg = "=== Invalid Address! ===";
        let mut entered_invalid = false;

        loop {
            let input = get_input_with_display(
                stdout,
                self.dims.0,
                title,
                prompt,
                Some(45),
                error_msg,
                entered_invalid,
            )?;

            // Parse the input
            break if self.net.is_server() {
                // If server enter port for 0.0.0.0:port.
                if input.is_empty() {
                    // Read config for port
                    let config = AppConfig::load(CONFIG_FILE).unwrap();
                    Ok(format!("0.0.0.0:{}", config.network.port))
                } else {
                    match input.parse::<u16>() {
                        Ok(port) if port > 0 => Ok(format!("0.0.0.0:{}", port)),
                        _ => {
                            entered_invalid = true;
                            continue;
                        }
                    }
                }
            } else {
                // If client, enter ip:port
                if input.is_empty() {
                    // Read config for address
                    let config = AppConfig::load(CONFIG_FILE).unwrap();
                    Ok(config.network.address())
                } else {
                    // Perform simple validation
                    let validation = input.split(':').collect::<Vec<_>>();
                    if validation.len() == 2
                        && validation[0].len() <= 39
                        && validation[1].len() <= 5
                    {
                        Ok(input)
                    } else {
                        entered_invalid = true;
                        continue;
                    }
                }
            };
        }
    }

    async fn draw(
        &self,
        last_history: &mut (&mut [Msg], &mut [Msg]),
        last_input: &mut String,
    ) -> Result<(), std::io::Error> {
        let mut stdout = std::io::stdout();

        let guard = self.state.read().await;
        let history = &guard.history;
        let history_slices = history.as_slices();
        let len_0 = history_slices.0.len();
        let len_1 = history_slices.1.len();
        if last_input != &self.input
            || &last_history.0[..len_0] != history_slices.0
            || &last_history.1[..len_1] != history_slices.1
        {
            // Clear the screen
            execute!(
                stdout,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
            )?;

            // Title
            execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;
            println!("Chat Room === {} === Ctrl+Q to quit", self.username);
            println!("{}", "─".repeat(self.dims.0 as usize));

            // Move cursor to correct position of chat area
            let msg_area = (self.dims.1 - 5) as usize;
            if history.len() < msg_area {
                execute!(
                    stdout,
                    crossterm::cursor::MoveTo(0, 2 + (msg_area - history.len()) as u16)
                )?;
            }

            // Messages
            for msg in history.iter().rev().take(msg_area).rev() {
                println!(
                    "   {}",
                    msg.to_chat().unwrap_or_else(|_| "<invalid>".into())
                );
            }

            // Draw the input line
            execute!(stdout, crossterm::cursor::MoveTo(0, self.dims.1 - 3))?;
            println!("{}", "─".repeat(self.dims.0 as usize));
            println!("> {}", self.input);

            // Move cursor to correct position
            execute!(
                stdout,
                crossterm::cursor::MoveTo(self.cursor_pos + 2, self.dims.1 - 2)
            )?;
            stdout.flush()?;

            if &last_history.0[..len_0] != history_slices.0 {
                last_history.0[..len_0].clone_from_slice(history_slices.0);
            }
            if &last_history.1[..len_1] != history_slices.1 {
                last_history.1[..len_1].clone_from_slice(history_slices.1);
            }
            if last_input != &self.input {
                *last_input = self.input.clone();
            }
        }
        Ok(())
    }
}

pub struct NetworkTask<
    N: al_crypto::NonceTrait,
    M: KeyedStorage<ConnId, TransportId>,
    R: KeyedStorage<TransportId, ConnId>,
    L: IndexedStorage<ConnId>,
> {
    driver: Arc<Mutex<Driver<Payload>>>,
    events: Arc<BoundaryQueue<NetEvent>>,
    net_in: Arc<BoundaryQueue<TaggedBytes>>,
    net_out: Arc<BoundaryQueue<TaggedBytes>>,
    token: CancellationToken,
    conn_map: M,
    tcp_to_conn: R,
    live: Arc<tokio::sync::RwLock<L>>,
    next_conn_id: ConnId,
    socket_tasks: JoinSet<()>,
    _phantom: std::marker::PhantomData<N>,
}

impl<
        N: al_crypto::NonceTrait,
        M: KeyedStorage<ConnId, TransportId>,
        R: KeyedStorage<TransportId, ConnId>,
        L: IndexedStorage<ConnId>,
    > NetworkTask<N, M, R, L>
{
    pub fn new(
        net_in: Arc<BoundaryQueue<TaggedBytes>>,
        net_out: Arc<BoundaryQueue<TaggedBytes>>,
        events: Arc<BoundaryQueue<NetEvent>>,
        token: CancellationToken,
    ) -> Self {
        Self {
            driver: Arc::new(Mutex::new(Driver::new())),
            events,
            net_in,
            net_out,
            token,
            conn_map: M::new(),
            tcp_to_conn: R::new(),
            live: Arc::new(tokio::sync::RwLock::new(L::new())),
            next_conn_id: 0,
            socket_tasks: JoinSet::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn add_tcp(
        &mut self,
        tcp: Tcp<N>,
        socket: TcpSocket,
        stream: tokio::net::TcpStream,
    ) -> Result<ConnId, DriverError> {
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;

        let tcp_id = {
            let mut driver = self.driver.lock().await;
            let tcp_id = driver.add_transport(tcp);

            let out = self.net_out.clone();
            let sink_id = driver.add_transport(FallibleSink::new(move |bytes: Payload| {
                out.send((conn_id, bytes))
                    .map_err(|_| al_transport::Backpressure::Closed)
            }));
            driver.connect(tcp_id, sink_id)?;
            tcp_id
        };
        self.conn_map
            .insert(conn_id, tcp_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;
        self.tcp_to_conn
            .insert(tcp_id, conn_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;
        self.live
            .write()
            .await
            .push(conn_id)
            .map_err(|e| DriverError::Transport(TransportError::Custom(Box::new(e))))?;

        let conn_token = self.token.clone();
        self.socket_tasks.spawn(async move {
            if let Err(e) = socket.run_with_cancel(stream, conn_token).await {
                eprintln!("Chat driver socket from `add_tcp` failed: {e}")
            }
        });
        Ok(conn_id)
    }

    pub async fn run(mut self, listener: Option<tokio::net::TcpListener>) {
        let notify = Arc::new(tokio::sync::Notify::new());
        let waker = std::task::Waker::from(Arc::new(TokioWaker::new(notify.clone())));

        loop {
            // Poll once per iteration. When the transports have data,
            // they fire the waker and the select below wakes to poll again.
            {
                let killed = {
                    self.driver
                        .lock()
                        .await
                        .poll(&mut std::task::Context::from_waker(&waker))
                };
                for tcp_id in killed {
                    let conn_id = match self.tcp_to_conn.get(&tcp_id) {
                        Ok(Some(&c)) => c,
                        Ok(None) => continue,
                        Err(e) => {
                            eprintln!("Network `tcp_to_conn get` failed: {e}");
                            continue;
                        }
                    };
                    if let Err(e) = self.conn_map.remove(&conn_id) {
                        eprintln!("Network `conn_map remove` failed: {e}");
                    }
                    if let Err(e) = self.tcp_to_conn.remove(&tcp_id) {
                        eprintln!("Network `tcp_to_conn remove` failed: {e}");
                    }
                    if let Err(e) = self.live.write().await.retain(|(_, &i)| i != conn_id) {
                        eprintln!("Network `live retain` failed: {e}");
                    }
                    if let Err(e) = self.events.send(NetEvent::Disconnected(conn_id)) {
                        eprintln!("Network `conn_map remove` failed: {e}");
                    }
                }
            }

            // Accept future (if server)
            let accept_fut = async {
                match &listener {
                    Some(l) => l.accept().await,
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                biased;
                _ = self.token.cancelled() => break,

                accepted = accept_fut => {
                    let (stream, _peer) = match accepted {
                        Ok(x) => x,
                        Err(e) => { eprintln!("Chat server `accept` error: {e}"); continue; }
                    };
                    let (tcp, socket) = match Tcp::<N>::new_responder(
                        al_secure::noise::handshake_pattern::HandshakePattern::NN,
                        Vec::new(), None, None
                    ) {
                        Ok(x) => x,
                        Err(e) => { eprintln!("Chat server `noise` error: {e}"); continue; }
                    };
                    match self.add_tcp(tcp, socket, stream).await {
                        Ok(conn_id) => if let Err(e) = self.events.send(NetEvent::Connected(conn_id)) {
                            eprintln!("Chat server events `send` error: {e}");
                            continue;
                        }
                        Err(e) => { eprintln!("Chat server `add_tcp` failed: {e}"); continue; }
                    }
                }

                item = self.net_in.recv() => {
                    use al_structures::traits::Downcast;
                    let mut driver = self.driver.lock().await;

                    let mut batch = Vec::new();
                    if let Ok(item) = item { batch.push(item); }
                    while let Ok(Some(extra)) = self.net_in.try_recv() { batch.push(extra); }

                    for (conn_id, bytes) in batch {
                        if conn_id == ConnId::MAX {
                            let msg = match DynMessage::from_format_slice(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    eprintln!("Chat server `broadcast` failed: {e}");
                                    continue;
                                }
                            };
                            let broadcast = match msg.downcast::<Broadcast>() {
                                Ok(bcst) => bcst,
                                Err(_) => {
                                    eprintln!("Chat server `broadcast downcast` failed.");
                                    continue;
                                }
                            };
                            let except = broadcast.except.unwrap_or_default();
                            let payload = broadcast.msg;

                            match self.live.read().await.values() {
                                Ok(values) => for live_id in values {
                                    if except.contains(&live_id) { continue; }
                                    if let Ok(Some(&tcp_id)) = self.conn_map.get(&live_id) {
                                        if let Err(e) = driver.deliver_to(tcp_id, payload.clone()) {
                                            eprintln!("Chat server `broadcast deliver_to` failed: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Chat server `live values` failed: {e}");
                                    continue;
                                }
                            }
                        } else {
                            match self.conn_map.get(&conn_id) {
                                Ok(Some(&tcp_id)) => {
                                    if let Err(e) = driver.deliver_to(tcp_id, bytes) {
                                        eprintln!("deliver_to failed: {e}");
                                    }
                                }
                                Ok(None) => {},
                                Err(e) => eprintln!("conn_map lookup failed: {e}"),
                            }
                        }
                    }
                    drop(driver);
                }

                _ = notify.notified() => {}
            }
        }

        // Attempt to join gracefully
        if tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while self.socket_tasks.join_next().await.is_some() {}
        })
        .await
        .is_err()
        {
            self.socket_tasks.shutdown().await;
        }
    }
}

pub struct NetworkHandle {
    format_id: al_events::FormatId,
    events: Arc<BoundaryQueue<NetEvent>>,
    app_out: Arc<BoundaryQueue<TaggedMsg>>,
    app_in: Arc<BoundaryQueue<TaggedMsg>>,
    _splices: Vec<SpliceHandle>,
    task: Mutex<Option<NetTask>>,
    handle: Mutex<Option<JoinHandle<()>>>,
    token: CancellationToken,
    mode: AtomicBool,
}

impl NetworkHandle {
    pub async fn new(format_id: al_events::FormatId) -> Self {
        let token = CancellationToken::new();
        let events = BoundaryQueue::new();
        let app_out = BoundaryQueue::new();
        let app_in = BoundaryQueue::new();
        let net_out = BoundaryQueue::new();
        let net_in = BoundaryQueue::new();

        let out_handle = splice_async(
            app_out.clone(),
            net_in.clone(),
            move |(id, msg): (ConnId, DynMessage)| {
                let mut bytes = Vec::new();
                msg.to_format(format_id, &mut bytes)
                    .expect("DynMessage serialization");
                (id, bytes)
            },
            |f| {
                tokio::spawn(f);
            },
            al_transport::splice::panic_on_error,
        )
        .await;

        let in_handle = splice_async(
            net_out.clone(),
            app_in.clone(),
            move |(id, bytes): (ConnId, Payload)| {
                (
                    id,
                    DynMessage::from_format_slice(&bytes).expect("DynMessage deserialization"),
                )
            },
            |f| {
                tokio::spawn(f);
            },
            al_transport::splice::panic_on_error,
        )
        .await;

        let task_events = events.clone();
        let task_token = token.clone();
        Self {
            format_id,
            events,
            app_out,
            app_in,
            _splices: vec![out_handle, in_handle],
            task: Mutex::new(Some(NetworkTask::new(
                net_in,
                net_out,
                task_events,
                task_token,
            ))),
            handle: Mutex::new(None),
            token,
            mode: AtomicBool::new(false),
        }
    }

    pub fn send_to(&self, conn_id: ConnId, msg: DynMessage) -> Result<(), TcpError> {
        self.app_out
            .send((conn_id, msg))
            .map_err(|_| TcpError::WireQueueClosed)
    }

    pub fn broadcast(&self, except: Option<Vec<ConnId>>, msg: DynMessage) -> Result<(), TcpError> {
        let mut bytes = Vec::new();
        msg.to_format(self.format_id, &mut bytes)?;
        self.app_out
            .send((ConnId::MAX, Broadcast::new(except, bytes).to_msg()))
            .map_err(|_| TcpError::WireQueueClosed)
    }

    pub fn is_server(&self) -> bool {
        self.mode.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn mark_server(&self) {
        self.mode.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

fn get_input_with_display(
    stdout: &mut std::io::Stdout,
    term_len: u16,
    title: &str,
    prompt: &str,
    max_len: Option<usize>,
    error_msg: &str,
    show_error_msg: bool,
) -> Result<String, TuiError> {
    let mut input = String::new();
    let mut cursor_pos = 0u16;
    let mut last_input = String::new();
    // Start with different last_cursor for first draw
    let mut last_cursor = 1u16;
    let mut last_error = show_error_msg;

    loop {
        // Only clear and redraw if input changed
        if input != last_input || cursor_pos != last_cursor || show_error_msg != last_error {
            execute!(
                stdout,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
            )?;
            execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;
            println!("{}", title);
            println!("{}", prompt);
            println!("{}", "─".repeat(term_len as usize));
            println!("{}", if show_error_msg { error_msg } else { "" });
            print!("> {}", input);
            stdout.flush()?;

            if input != last_input {
                last_input = input.clone();
            }
            last_cursor = cursor_pos;
            last_error = show_error_msg;
        }

        // Position cursor
        execute!(stdout, crossterm::cursor::MoveTo(cursor_pos + 2, 4))?;

        // Wait for input with timeout
        if crossterm::event::poll(Duration::from_millis(200))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    if key.kind != crossterm::event::KeyEventKind::Release {
                        match key.code {
                            KeyCode::Enter => {
                                break;
                            }
                            KeyCode::Backspace => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                    input.remove(cursor_pos as usize);
                                }
                            }
                            KeyCode::Left => {
                                cursor_pos = cursor_pos.saturating_sub(1);
                            }
                            KeyCode::Right => {
                                if (cursor_pos as usize) < input.len() {
                                    cursor_pos += 1;
                                }
                            }
                            KeyCode::Home => {
                                cursor_pos = 0;
                            }
                            KeyCode::End => {
                                cursor_pos = input.len() as u16;
                            }
                            KeyCode::Char(c) => {
                                if let Some(max) = max_len {
                                    if input.len() >= max {
                                        continue;
                                    }
                                }
                                input.insert(cursor_pos as usize, c);
                                cursor_pos += 1;
                            }
                            KeyCode::Esc => Err(std::io::Error::new(
                                std::io::ErrorKind::Interrupted,
                                "User cancelled input",
                            ))?,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(input)
}

pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
}

impl NetworkConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub network: NetworkConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                ip: "127.0.0.1".to_string(),
                port: 7878,
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
                toml::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            )
        }
    }

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), String> {
        std::fs::write(
            path,
            toml::to_string_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum TuiError {
    IoError(std::io::Error),
    TcpError(TcpError),
    DispatcherError(al_transport::dispatcher::DispatcherError),
    DriverError(DriverError),
    AppError(String),
}

impl From<std::io::Error> for TuiError {
    fn from(value: std::io::Error) -> Self {
        TuiError::IoError(value)
    }
}

impl From<TcpError> for TuiError {
    fn from(value: TcpError) -> Self {
        TuiError::TcpError(value)
    }
}

impl From<al_transport::dispatcher::DispatcherError> for TuiError {
    fn from(value: al_transport::dispatcher::DispatcherError) -> Self {
        TuiError::DispatcherError(value)
    }
}

impl From<DriverError> for TuiError {
    fn from(value: DriverError) -> Self {
        TuiError::DriverError(value)
    }
}
