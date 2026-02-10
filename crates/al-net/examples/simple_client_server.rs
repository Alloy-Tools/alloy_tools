use std::{
    collections::VecDeque,
    fs::{self},
    io::Write,
    sync::Arc,
    time::Duration,
};

use al_core::{event, register_event, Buffered, Command, Event, Publisher, Queue, Transport};
use al_crypto::{Monotonic, NonceTrait};
use al_net::{CommandDispatcher, ConnectionManager, Tcp, TcpError};
use crossterm::{
    event::{KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
}

impl NetworkConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

struct App<N: NonceTrait> {
    // Terminal
    dims: (u16, u16),
    stop: bool,
    // Communication
    inbound: Arc<Queue<TcpMsg>>,
    outbound: Arc<Buffered<Command>>,
    publisher_ref: Arc<Publisher<Command>>,
    dispatcher: Option<Arc<CommandDispatcher<(u64, Arc<ConnectionManager<N>>), Arc<RwLock<Self>>>>>,
    connections: Arc<ConnectionManager<N>>,
    id: u64,
    tcp_handle: Option<tokio::task::JoinHandle<Result<(), al_net::TcpError>>>,
    // Chat
    username: String,
    input: String,
    cursor_pos: u16,
    history: VecDeque<Msg>,
    max_history_len: u8,
}

impl<N: NonceTrait> App<N> {
    pub async fn new(width: u16, height: u16) -> Arc<RwLock<Self>> {
        let publisher = Arc::new(Publisher::new());
        let app = Arc::new(RwLock::new(Self {
            dims: (width, height),
            stop: false,
            inbound: Arc::new(Queue::new()),
            outbound: Arc::new(Buffered::new(publisher.clone())),
            publisher_ref: publisher,
            dispatcher: None,
            connections: Arc::new(ConnectionManager::new()),
            id: 0,
            tcp_handle: None,
            username: String::new(),
            input: String::new(),
            cursor_pos: 0,
            history: VecDeque::with_capacity(150),
            max_history_len: 100,
        }));

        let mut dispatcher = CommandDispatcher::new(app.clone());

        // Register `Msg` handler to store messages
        dispatcher
            .register_event(|_, app, event: Msg| async move {
                let mut guard = app.write().await;
                guard.history.push_back(event);
                if guard.history.len() > guard.max_history_len as usize {
                    guard.history.pop_front();
                }
            })
            .await;

        app.write().await.dispatcher = Some(Arc::new(dispatcher));

        app
    }

    /// Blocks the thread running a loop checking for events and drawing UI. Should be spawned on another thread.
    pub async fn run_tui(state: Arc<RwLock<Self>>) -> Result<(), TuiError> {
        // Set up terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        {
            let mut state = state.write().await;
            state.get_user_name(&mut stdout)?;
            state.start_client_or_server(&mut stdout).await?;
        }

        // Start with non-empty last_input to trigger first draw
        let mut slice_0 = vec![Msg::new(Vec::new(), Vec::new()); 150];
        let mut slice_1 = vec![Msg::new(Vec::new(), Vec::new()); 150];
        let mut last_history = (slice_0.as_mut_slice(), slice_1.as_mut_slice());
        let mut last_input = ".".to_string();
        loop {
            let (dispatcher, connections, inbound) = {
                let state_guard = state.read().await;
                if state_guard.stop {
                    break;
                }
                let dispatcher = if let Some(dispatcher) = &state_guard.dispatcher {
                    dispatcher
                } else {
                    break;
                };
                (
                    dispatcher.clone(),
                    state_guard.connections.clone(),
                    state_guard.inbound.clone(),
                )
            };

            // Check for network events
            match inbound.recv_avaliable().await {
                Ok(updates) => {
                    for update in updates {
                        dispatcher
                            .dispatch((update.0, connections.clone()), update.1)
                            .await;
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving from TCP connection: {:?}", e);
                    break;
                }
            }

            // Check for terminal events
            if crossterm::event::poll(Duration::from_millis(200))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.kind != crossterm::event::KeyEventKind::Release {
                        state.write().await.handle_key(key).await?;
                    }
                }
            }

            state
                .read()
                .await
                .draw(&mut last_history, &mut last_input)?;
        }

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;

        if let Some(handle) = state.write().await.tcp_handle.take() {
            match handle.await {
                Ok(r) => {
                    if let Err(e) = r {
                        eprint!("TCP thread error: {}", e)
                    }
                }
                Err(e) => eprintln!("TCP thread join error: {}", e),
            }
        }

        Ok(())
    }

    fn get_user_name(&mut self, stdout: &mut std::io::Stdout) -> Result<(), TuiError> {
        let title =
            "Tic-Tac-Toe === Choose Username === Max 10 characters, UTF8 only === Ctrl+Q to quit";
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

    fn get_client_server(&mut self, stdout: &mut std::io::Stdout) -> Result<bool, TuiError> {
        let title = "Tic-Tac-Toe === Choose client or server === Ctrl+Q to quit";
        let prompt = "Start as (c)lient or (s)erver? [s/c]: ";
        let error_msg = "=== Invalid Choice! ===";
        let mut entered_invalid = false;
        loop {
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
                "s" => break Ok(true),
                "c" => break Ok(false),
                _ => {
                    entered_invalid = true;
                    continue;
                }
            }
        }
    }

    fn get_address(
        &mut self,
        server: bool,
        stdout: &mut std::io::Stdout,
    ) -> Result<String, TuiError> {
        let title = "Tic-Tac-Toe === Enter address === Ctrl+Q to quit";
        let prompt = if server {
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
            break if server {
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

    async fn start_client_or_server(
        &mut self,
        stdout: &mut std::io::Stdout,
    ) -> Result<(), TuiError> {
        let app_dispatcher = if let Some(d) = &self.dispatcher {
            d.clone()
        } else {
            Err(TuiError::AppError("No dispatcher".to_string()))?
        };
        let server = self.get_client_server(stdout)?;
        let addr = self.get_address(server, stdout)?;

        // Setup common server/client handlers
        let mut dispatcher =
            CommandDispatcher::<(u64, Arc<ConnectionManager<N>>), _>::new(self.inbound.clone());
        dispatcher
            .register_command(|(conn_id, _), inbound, cmd| async move {
                inbound
                    .send(TcpMsg(conn_id, cmd))
                    .await
                    .expect("TUI inbound queue failed.");
            })
            .await;

        let connection_manager = Arc::new(ConnectionManager::new());
        if server {
            let token = Arc::new(RwLock::new(false));
            let token_clone = token.clone();

            //TODO: just place in the outbound queue as well
            // Forward recieved messsages to all other connected clients
            dispatcher
                .register_command(|(sender_id, connections), _, cmd| async move {
                    for (id, conn) in connections
                        .with_connections(|conns| {
                            conns
                                .iter()
                                .filter(|(&id, _)| id != sender_id)
                                .map(|(&id, tcp)| (id, tcp.clone()))
                                .collect::<Vec<_>>()
                        })
                        .await
                    {
                        if let Err(e) = conn.send(cmd.clone()).await {
                            eprintln!(
                                "Failed to forward message from conn {} to conn {}: {:?}",
                                sender_id, id, e
                            );
                        }
                    }
                })
                .await;

            let publisher = self.publisher_ref.clone();
            let local_id = self.id;
            let server_handle = tokio::spawn(async move {
                Tcp::<N>::run_server_with_shutdown(
                    addr,
                    Some(Duration::from_millis(500)),
                    al_net::HandshakePattern::NN,
                    Vec::new(),
                    None,
                    None,
                    token_clone,
                    |tcp| {
                        let app_dispatcher_clone = app_dispatcher.clone();
                        let dispatcher_clone = dispatcher.clone();
                        let connection_manager_clone = connection_manager.clone();
                        let publisher_clone = publisher.clone();
                        tokio::spawn(async move {
                            let tcp = Arc::new(tcp);
                            let conn_id = connection_manager_clone.insert(tcp.clone()).await;

                            if let Err(e) = publisher_clone.subscribe(tcp.clone()) {
                                eprintln!(
                                    "Failed to subscribe new client {} to outbound publisher: {:?}",
                                    conn_id, e
                                );
                                connection_manager_clone.remove(&conn_id).await;
                                return;
                            }

                            app_dispatcher_clone
                                .dispatch(
                                    (local_id, connection_manager_clone.clone()),
                                    Msg::new(
                                        "Server",
                                        format!("New client with id {} connected!", conn_id),
                                    )
                                    .to_cmd(),
                                )
                                .await;

                            loop {
                                tokio::task::yield_now().await;

                                match tcp.recv().await {
                                    Ok(cmd) => {
                                        dispatcher_clone
                                            .dispatch(
                                                (conn_id, connection_manager_clone.clone()),
                                                cmd,
                                            )
                                            .await
                                    }
                                    Err(e) => {
                                        eprintln!("Client {} disconnected: {:?}", conn_id, e);
                                        connection_manager_clone.remove(&conn_id).await;
                                        app_dispatcher_clone
                                            .dispatch(
                                                (local_id, connection_manager_clone.clone()),
                                                Msg::new(
                                                    "Server",
                                                    format!(
                                                        "Client with id {} disconnected!",
                                                        conn_id
                                                    ),
                                                )
                                                .to_cmd(),
                                            )
                                            .await;
                                        break;
                                    }
                                }
                            }
                        });
                    },
                )
                .await
            });

            self.tcp_handle = Some(server_handle);
        } else {
            // if client connect to server with address
            let tcp = Arc::new(
                Tcp::<N>::connect(
                    addr,
                    None,
                    al_net::HandshakePattern::NN,
                    Vec::new(),
                    None,
                    None,
                )
                .await?,
            );
            let conn_id = connection_manager.insert(tcp.clone()).await;

            if let Err(e) = self.publisher_ref.subscribe(tcp.clone()) {
                let msg = format!(
                    "Failed to subscribe server with id {} to outbound publisher: {:?}",
                    conn_id, e
                );
                eprintln!("{}", msg);
                self.connections.remove(&conn_id).await;
                Err(TuiError::AppError(msg))?
            }

            let _ = self.inbound.send(TcpMsg(self.id, Msg::new("Client", format!("Connected to server {}!", conn_id)).to_cmd())).await;

            let local_id = self.id;
            let connection_clone = self.connections.clone();
            let app_dispatcher_clone = app_dispatcher.clone();
            let client_handle = tokio::spawn(async move {
                loop {
                    tokio::task::yield_now().await;

                    match tcp.recv().await {
                        Ok(cmd) => {
                            dispatcher
                                .dispatch((conn_id, connection_manager.clone()), cmd)
                                .await
                        }
                        Err(e) => {
                            eprintln!("Disconnected from server {}: {:?}", conn_id, e);
                            connection_clone.remove(&conn_id).await;
                            app_dispatcher_clone
                                .dispatch(
                                    (local_id, connection_clone.clone()),
                                    Msg::new("Client", format!("Client disconnected from server!"))
                                        .to_cmd(),
                                )
                                .await;
                            break;
                        }
                    }
                }

                Ok(())
            });

            self.tcp_handle = Some(client_handle);
        }
        Ok(())
    }

    fn draw(
        &self,
        last_history: &mut (&mut [Msg], &mut [Msg]),
        last_input: &mut String,
    ) -> Result<(), std::io::Error> {
        let mut stdout = std::io::stdout();

        let history_slices = self.history.as_slices();
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
            if self.history.len() < msg_area {
                execute!(
                    stdout,
                    crossterm::cursor::MoveTo(0, 2 + (msg_area - self.history.len()) as u16)
                )?;
            }

            // Messages
            for msg in self.history.iter().rev().take(msg_area).rev() {
                println!("   {}", msg.to_chat().unwrap());
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

    async fn handle_key(&mut self, key: KeyEvent) -> Result<(), std::io::Error> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL {
                    match c {
                        'c' | 'q' => self.stop = true,
                        _ => {}
                    }
                } else {
                    self.input.insert(self.cursor_pos as usize, c);
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos as usize);
                }
            }
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                if (self.cursor_pos as usize) < self.input.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    let msg = self.input.trim();
                    let msg = Msg::new(self.username.as_bytes(), msg.as_bytes());
                    if let Err(e) = self.outbound.send(msg.clone().to_cmd()).await {
                        eprintln!("Failed to send message: {:?}", e);
                        if let Some(dispatcher) = &self.dispatcher {
                            dispatcher
                                .dispatch(
                                    (self.id, self.connections.clone()),
                                    Msg::new("Client", format!("Failed to send message!")).to_cmd(),
                                )
                                .await;
                        }
                    } else {
                        self.history.push_back(msg);
                        if self.history.len() > self.max_history_len as usize {
                            self.history.pop_front();
                        }
                        self.input.clear();
                        self.cursor_pos = 0;
                    }
                }
            }
            KeyCode::Esc => self.stop = true,
            _ => {}
        }
        Ok(())
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

#[derive(Debug)]
pub enum TuiError {
    IoError(std::io::Error),
    TcpError(TcpError),
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

#[event]
pub struct TcpMsg(u64, Command);

#[event]
pub struct Msg(Vec<u8>, Vec<u8>);

impl Msg {
    pub fn new(name: impl Into<Vec<u8>>, msg: impl Into<Vec<u8>>) -> Self {
        Self(name.into(), msg.into())
    }

    pub fn to_chat(&self) -> Result<String, std::str::Utf8Error> {
        match str::from_utf8(&self.0) {
            Ok(name) => match str::from_utf8(&self.1) {
                Ok(msg) => Ok(format!("{}: {}", name, msg)),
                Err(e) => Err(e)?,
            },
            Err(e) => Err(e)?,
        }
    }
}

#[tokio::main]
async fn main() {
    // Register events
    register_event!(TcpMsg);
    register_event!(Msg);

    let tui_handle = tokio::task::spawn_blocking(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let app = {
                let (width, height) = crossterm::terminal::size().unwrap();
                App::<Monotonic>::new(width, height).await
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
