use std::{
    fs::{self},
    io::Write,
    sync::Arc,
    time::Duration,
};

use al_core::{event, register_event, Command, Event, Link, Queue, Transport};
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

struct App<N: NonceTrait> {
    // Terminal
    dims: (u16, u16),
    stop: bool,
    // Communication
    inbound: Arc<Queue<TuiUpdate>>,
    outbound: Arc<Queue<Command>>,
    dispatcher: Option<CommandDispatcher<N, Arc<RwLock<Self>>>>,
    connections: Arc<ConnectionManager<N>>,
    tcp_handle: Option<tokio::task::JoinHandle<Result<(), al_net::TcpError>>>,
    // Chat
    username: String,
    input: String,
    cursor_pos: u16,
    history: Vec<Msg>,
    // Game
    first: bool,
    turn: bool,
    board: [u8; 9],
}

impl<N: NonceTrait> App<N> {
    pub async fn new(width: u16, height: u16) -> Arc<RwLock<Self>> {
        let app = Arc::new(RwLock::new(Self {
            dims: (width, height),
            stop: false,
            inbound: Arc::new(Queue::new()),
            outbound: Arc::new(Queue::new()),
            dispatcher: None,
            connections: Arc::new(ConnectionManager::new()),
            tcp_handle: None,
            username: String::new(),
            input: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            first: false,
            turn: false,
            board: [0u8; 9],
        }));

        let mut dispatcher = CommandDispatcher::new(app.clone());

        // Register `Msg` handler to store messages
        dispatcher
            .register_event(|_, _, app, event: Msg| async move {
                app.write().await.history.push(event);
            })
            .await;
        // Register `MakeMove` handler to update the board
        dispatcher
            .register_event(|_, _, app, event: MakeMove| async move {
                let mut app = app.write().await;
                let opp_val = if app.first { 2 } else { 1 };
                if event.0 < 9 && app.board[event.0] == 0 {
                    app.board[event.0] = opp_val;
                    app.turn = true;
                }
            })
            .await;

        app.write().await.dispatcher = Some(dispatcher);

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

        // Start with non-empty last_board to trigger first draw
        let mut last_board = [10u8; 9];
        let mut last_history = Vec::new();
        let mut last_input = String::new();
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
                            .dispatch(update.0, connections.clone(), update.1)
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
                .draw(&mut last_board, &mut last_history, &mut last_input)?;
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
        let server = self.get_client_server(stdout)?;
        let addr = self.get_address(server, stdout)?;

        // Setup common server/client handlers
        let mut dispatcher = CommandDispatcher::new(self.inbound.clone());
        dispatcher
            .register_command(|conn_id, _, inbound, cmd| async move {
                inbound
                    .send(TuiUpdate(conn_id, cmd))
                    .await
                    .expect("TUI inbound queue failed.");
            })
            .await;

        let connection_manager = Arc::new(ConnectionManager::new());
        if server {
            self.turn = true;
            let token = Arc::new(RwLock::new(false));
            let token_clone = token.clone();
            let outbound_clone = self.outbound.clone();

            let server_handle = tokio::spawn(async move {
                Tcp::<Monotonic>::run_server_with_shutdown(
                    addr,
                    Some(Duration::from_millis(500)),
                    al_net::HandshakePattern::NN,
                    Vec::new(),
                    None,
                    None,
                    token_clone,
                    |tcp| {
                        let dispatcher_clone = dispatcher.clone();
                        let connection_manager_clone = connection_manager.clone();
                        let outbound_clone = outbound_clone.clone();
                        tokio::spawn(async move {
                            let tcp = Arc::new(tcp);
                            let conn_id = connection_manager_clone.insert(tcp.clone()).await;

                            dispatcher_clone
                                .dispatch_event(
                                    conn_id,
                                    connection_manager_clone.clone(),
                                    Box::new(Msg::new(
                                        "Server",
                                        format!("New client with id {} connected!", conn_id),
                                    )),
                                )
                                .await;

                            let outbound_link = Link::new(outbound_clone, tcp.clone());

                            loop {
                                tokio::task::yield_now().await;

                                match tcp.recv().await {
                                    Ok(cmd) => {
                                        dispatcher_clone
                                            .dispatch(
                                                conn_id,
                                                connection_manager_clone.clone(),
                                                cmd,
                                            )
                                            .await
                                    }
                                    Err(e) => {
                                        eprintln!("Client {} disconnected: {:?}", conn_id, e);
                                        outbound_link.link_task().write().await.abort();
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
                Tcp::<Monotonic>::connect(
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

            let outbound_clone = self.outbound.clone();
            let client_handle = tokio::spawn(async move {
                let outbound_link = Link::new(outbound_clone, tcp.clone());

                loop {
                    tokio::task::yield_now().await;

                    match tcp.recv().await {
                        Ok(cmd) => {
                            dispatcher
                                .dispatch(conn_id, connection_manager.clone(), cmd)
                                .await
                        }
                        Err(e) => {
                            eprintln!("Disconnected from server: {:?}", e);
                            outbound_link.link_task().write().await.abort();
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
        last_board: &mut [u8; 9],
        last_history: &mut Vec<Msg>,
        last_input: &mut String,
    ) -> Result<(), std::io::Error> {
        let mut stdout = std::io::stdout();

        if last_board != &self.board || last_history != &self.history || last_input != &self.input {
            // Clear the screen
            execute!(
                stdout,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
            )?;

            // Title
            execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;
            println!("Tic-Tac-Toe === Ctrl+Q to quit");
            println!("{}", "─".repeat(self.dims.0 as usize));

            // Tic Tac Toe
            println!(
                "{}|{}|{}",
                get_char(self.board[0]),
                get_char(self.board[1]),
                get_char(self.board[2])
            );
            println!(
                "{}|{}|{}",
                get_char(self.board[3]),
                get_char(self.board[4]),
                get_char(self.board[5])
            );
            println!(
                "{}|{}|{}",
                get_char(self.board[6]),
                get_char(self.board[7]),
                get_char(self.board[8])
            );
            println!("{}", "─".repeat(self.dims.0 as usize));

            // Messages
            println!("Chat:");
            for msg in self
                .history
                .iter()
                .rev()
                .take((self.dims.1 - 8) as usize)
                .rev()
            {
                println!("   {}", msg.to_chat().unwrap());
            }

            // Draw the input line
            execute!(stdout, crossterm::cursor::MoveTo(0, self.dims.1 - 2))?;
            println!("{}", "─".repeat(self.dims.0 as usize));
            println!("> {}", self.input);

            // Move cursor to correct position
            execute!(
                stdout,
                crossterm::cursor::MoveTo(self.cursor_pos + 2, self.dims.1 - 2)
            )?;
            stdout.flush()?;

            *last_board = self.board;
            if last_history != &self.history {
                *last_history = self.history.clone();
            }
            if last_input != &self.input {
                *last_input = self.input.clone();
            }
        }
        Ok(())
    }

    fn make_move(&mut self, space: usize) -> Result<(), ()> {
        // Check if the spot is already filled
        if self.turn && self.board[space] == 0 {
            // Fill the spot
            self.board[space] = if self.first { 1 } else { 2 };
            self.turn = false;
            Ok(())
        } else {
            Err(())
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<(), std::io::Error> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL {
                    match c {
                        'c' | 'q' => self.stop = true,
                        '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                            let pos = c.to_digit(10).unwrap() as usize - 1;
                            if let Ok(_) = self.make_move(pos) {
                                if let Err(e) = self.outbound.send(MakeMove(pos).to_cmd()).await {
                                    eprintln!("Failed to send MakeMove: {:?}", e)
                                }
                            }
                        }
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
                        eprintln!("Failed to send message: {:?}", e)
                    }
                    self.history.push(msg);
                    self.input.clear();
                    self.cursor_pos = 0;
                }
            }
            KeyCode::Esc => self.stop = true,
            _ => {}
        }
        Ok(())
    }
}

fn get_char(n: u8) -> char {
    match n {
        1 => 'X',
        2 => 'O',
        _ => ' ',
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
pub struct TuiUpdate(u64, Command);

#[event]
pub struct MakeMove(usize);

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
    register_event!(TuiUpdate);
    register_event!(MakeMove);
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
