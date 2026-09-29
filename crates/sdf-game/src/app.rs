use crate::{
    block_on,
    game::{GameState, HealthUpdate, PlayerUpdate, ProjectileSpawn, Scoreboard},
    net::{AnyStore, Dispatcher, KeyedStore, NetEvent, NetworkState},
    parse_addr_from_str,
    render::RenderState,
    CLIENT_ADDR,
};
use al_events::FormatId;
use std::{process::Child, sync::Arc, time::Duration};
use winit::{
    application::ApplicationHandler, dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop,
    window::Window,
};

const TITLE: &str = "SDF Game MVP";
const START_SIZE: LogicalSize<i32> = LogicalSize::new(800, 600);
const SEND_INTERVAL: Duration = Duration::from_millis(50);

pub enum AppState {
    MainMenu(MenuState),
    InGame {
        game: GameState,
        dispatcher: Dispatcher,
    },
}

impl AppState {
    pub async fn new_game(
        net_state: NetworkState,
        send_interval: std::time::Duration,
        any: AnyStore,
        keyed: KeyedStore,
    ) -> Self {
        let dispatcher = Dispatcher::new(net_state.state().clone(), any, keyed);

        // ----- Setup dispatcher -----
        // Peer positions
        dispatcher
            .register_event(|_, state, update: PlayerUpdate| {
                Box::pin(async move {
                    // Record for next game loop
                    state.write().await.upsert_player(
                        update.conn_id(),
                        update.pos(),
                        update.radius(),
                    );
                })
            })
            .await
            .unwrap();

        // Projectile Spawning
        dispatcher
            .register_event(|_, state, spawn: ProjectileSpawn| {
                Box::pin(async move {
                    state
                        .write()
                        .await
                        .queue_projectile(spawn.owner(), spawn.pos(), spawn.dir());
                })
            })
            .await
            .unwrap();

        // Health Updates
        dispatcher
            .register_event(|_, state, update: HealthUpdate| {
                Box::pin(async move {
                    state
                        .write()
                        .await
                        .queue_health(update.conn_id(), update.hp());
                })
            })
            .await
            .unwrap();

        // Scoreboard Updates
        dispatcher
            .register_event(|_, state, sb: Scoreboard| {
                Box::pin(async move {
                    state.write().await.set_scoreboard(sb);
                })
            })
            .await
            .unwrap();

        // Connection lifecycle
        dispatcher
            .register_event(|ctx, state, evt: NetEvent| {
                Box::pin(async move {
                    match evt {
                        NetEvent::NoOp => {}
                        NetEvent::Connected(id) => state.write().await.set_local_conn(Some(id)),
                        NetEvent::Welcome(id) => state.write().await.set_server_id(Some(id)),
                        NetEvent::Disconnected(id) => {
                            let mut s = state.write().await;
                            if ctx.source().is_none() && s.local_conn() == Some(id) {
                                // Own conn dropped
                                s.set_local_conn(None);
                                eprintln!("Lost connection to server.");
                                //TODO: transition back to main menu
                            } else {
                                // Peer disconnected
                                s.remove_player(id);
                            }
                        }
                    }
                })
            })
            .await
            .unwrap();

        Self::InGame {
            game: GameState::new(send_interval, net_state),
            dispatcher,
        }
    }
}

pub struct MenuState {
    screen: Option<MenuScreen>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            screen: Some(MenuScreen::new_main()),
        }
    }

    pub fn set_screen(&mut self, screen: MenuScreen) {
        self.screen = Some(screen);
    }

    pub fn screen(&self) -> &Option<MenuScreen> {
        &self.screen
    }

    pub fn screen_mut(&mut self) -> &mut Option<MenuScreen> {
        &mut self.screen
    }

    pub fn tick_anim(&mut self) {
        let Some(MenuScreen::Main {
            selected,
            anim,
            anim_last,
            ..
        }) = &mut self.screen
        else {
            return;
        };

        let now = std::time::Instant::now();
        let dt = (now - *anim_last).as_secs_f32().min(0.1);
        *anim_last = now;

        let factor = 1. - (-14. * dt).exp();
        for (i, a) in anim.iter_mut().enumerate() {
            let target = if i == *selected { 1. } else { 0. };
            *a += (target - *a) * factor;
        }
    }
}

pub enum MenuScreen {
    Main {
        selected: usize,
        items: Vec<MenuAction>,
        anim: Vec<f32>,
        anim_last: std::time::Instant,
    },
    ClientAddr {
        addr_input: String,
        blink: std::time::Instant,
    },
}

impl MenuScreen {
    pub fn new_main() -> Self {
        Self::Main {
            selected: 0,
            items: vec![
                MenuAction::Client {
                    addr: String::new(),
                },
                MenuAction::Host,
                MenuAction::Quit,
            ],
            anim: vec![1., 0., 0.],
            anim_last: std::time::Instant::now(),
        }
    }

    pub fn new_client() -> Self {
        Self::ClientAddr {
            addr_input: String::new(),
            blink: std::time::Instant::now(),
        }
    }
}

#[derive(Clone)]
pub enum MenuAction {
    Client { addr: String },
    Host,
    Quit,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(mut child) = self.server.take() {
            let _ = child.kill();
        }
    }
}

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    format_id: FormatId,
    state: AppState,
    server: Option<std::process::Child>,
}

impl App {
    pub async fn new_client(
        format_id: FormatId,
        addr: String,
        send_interval: std::time::Duration,
        any: crate::net::AnyStore,
        keyed: crate::net::KeyedStore,
    ) -> Self {
        Self {
            window: None,
            render_state: None,
            format_id,
            state: AppState::new_game(
                NetworkState::new_client(format_id, addr),
                send_interval,
                any,
                keyed,
            )
            .await,
            server: None,
        }
    }

    pub async fn new_host(
        format_id: FormatId,
        send_interval: std::time::Duration,
        any: crate::net::AnyStore,
        keyed: crate::net::KeyedStore,
    ) -> Self {
        let server = Some(Self::spawn_server());
        Self::wait_for_server(CLIENT_ADDR);
        Self {
            window: None,
            render_state: None,
            format_id,
            state: AppState::new_game(
                NetworkState::new_client(format_id, CLIENT_ADDR.into()),
                send_interval,
                any,
                keyed,
            )
            .await,
            server,
        }
    }

    pub fn new_menu(format_id: FormatId) -> Self {
        Self {
            window: None,
            render_state: None,
            format_id,
            state: AppState::MainMenu(MenuState::new()),
            server: None,
        }
    }

    pub fn run(&mut self) -> Result<(), winit::error::EventLoopError> {
        EventLoop::new().unwrap().run_app(self)
    }

    pub fn set_server(&mut self, server: Child) {
        if let Some(mut old_server) = self.server.replace(server) {
            let _ = old_server.kill();
        }
    }

    fn spawn_server() -> Child {
        std::process::Command::new(std::env::current_exe().expect("current exe"))
            .arg("server")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("Failed to spawn server subprocess")
    }

    fn wait_for_server(addr: &str) {
        // wait for server to start accepting connections
        let mut connected = false;
        for _ in 0..30 {
            if std::net::TcpStream::connect(addr).is_ok() {
                connected = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !connected {
            eprintln!("Server subprocess didn't start listening in time.");
            std::process::exit(1);
        }
    }

    fn transition_to(&mut self, action: MenuAction) {
        match action {
            MenuAction::Client { addr } => {
                println!("Connecting to server at '{addr}'");
                let state = block_on(AppState::new_game(
                    NetworkState::new_client(self.format_id, addr),
                    SEND_INTERVAL,
                    Default::default(),
                    Default::default(),
                ));
                self.state = state;
            }
            MenuAction::Host => {
                self.set_server(Self::spawn_server());
                Self::wait_for_server(CLIENT_ADDR);
                let state = block_on(AppState::new_game(
                    NetworkState::new_client(self.format_id, CLIENT_ADDR.into()),
                    SEND_INTERVAL,
                    Default::default(),
                    Default::default(),
                ));
                self.state = state;
            }
            MenuAction::Quit => std::process::exit(0),
        }
    }

    fn menu_event(
        &mut self,
        event: winit::event::WindowEvent,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Option<MenuAction> {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(render_state) = &mut self.render_state {
                    if size.width > 0 && size.height > 0 {
                        render_state.config.width = size.width;
                        render_state.config.height = size.height;
                        render_state
                            .surface
                            .configure(&render_state.device, &render_state.config);
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(!focused);
                }
            }
            WindowEvent::RedrawRequested => {
                if let AppState::MainMenu(menu_state) = &mut self.state {
                    menu_state.tick_anim();
                    if let Some(state) = &mut self.render_state {
                        state.render_menu(menu_state);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Only catch on release
                if event.state == winit::event::ElementState::Pressed {
                    return None;
                }

                let AppState::MainMenu(menu_state) = &mut self.state else {
                    return None;
                };

                match menu_state.screen.take().unwrap() {
                    MenuScreen::Main {
                        mut selected,
                        items,
                        anim,
                        anim_last,
                    } => {
                        if let winit::keyboard::PhysicalKey::Code(c) = event.physical_key {
                            use winit::keyboard::KeyCode;
                            match c {
                                KeyCode::Space | KeyCode::Enter => {
                                    if let Some(action) = items.get(selected).cloned() {
                                        if let MenuAction::Client { .. } = action {
                                            menu_state.set_screen(MenuScreen::new_client());
                                            return None;
                                        } else {
                                            return Some(action);
                                        }
                                    }
                                }
                                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                                    if !items.is_empty() {
                                        selected = if selected == 0 {
                                            items.len() - 1
                                        } else {
                                            selected - 1
                                        };
                                    }
                                    menu_state.set_screen(MenuScreen::Main {
                                        selected,
                                        items,
                                        anim,
                                        anim_last,
                                    });
                                    return None;
                                }
                                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                                    if !items.is_empty() {
                                        selected = (selected + 1) % items.len();
                                    }
                                    menu_state.set_screen(MenuScreen::Main {
                                        selected,
                                        items,
                                        anim,
                                        anim_last,
                                    });
                                    return None;
                                }
                                _ => {}
                            }
                        }

                        if let winit::keyboard::Key::Character(s) = &event.logical_key {
                            match s.to_lowercase().as_str() {
                                "w" | "a" => {
                                    if !items.is_empty() {
                                        selected = if selected == 0 {
                                            items.len() - 1
                                        } else {
                                            selected - 1
                                        };
                                    }
                                    menu_state.set_screen(MenuScreen::Main {
                                        selected,
                                        items,
                                        anim,
                                        anim_last,
                                    });
                                    return None;
                                }
                                "s" | "d" => {
                                    if !items.is_empty() {
                                        selected = (selected + 1) % items.len();
                                    }
                                    menu_state.set_screen(MenuScreen::Main {
                                        selected,
                                        items,
                                        anim,
                                        anim_last,
                                    });
                                    return None;
                                }
                                _ => {}
                            }
                        }
                        menu_state.set_screen(MenuScreen::Main {
                            selected,
                            items,
                            anim,
                            anim_last,
                        });
                    }
                    MenuScreen::ClientAddr {
                        mut addr_input,
                        blink,
                    } => {
                        if let winit::keyboard::PhysicalKey::Code(c) = event.physical_key {
                            use winit::keyboard::KeyCode;
                            match c {
                                KeyCode::Enter => {
                                    let addr = if addr_input.is_empty() {
                                        CLIENT_ADDR.to_string()
                                    } else {
                                        match parse_addr_from_str(addr_input.trim().to_string()) {
                                            Some(a) => a,
                                            None => {
                                                menu_state.set_screen(MenuScreen::ClientAddr {
                                                    addr_input,
                                                    blink,
                                                });
                                                return None;
                                            }
                                        }
                                    };
                                    menu_state.set_screen(MenuScreen::new_main());
                                    return Some(MenuAction::Client { addr });
                                }
                                KeyCode::Escape => {
                                    menu_state.set_screen(MenuScreen::new_main());
                                    return None;
                                }
                                KeyCode::Backspace => {
                                    addr_input.pop();
                                }
                                _ => {}
                            }
                            if let winit::keyboard::Key::Character(s) = &event.logical_key {
                                // Filter allowed keys
                                for c in s.chars() {
                                    if c.is_ascii_alphanumeric() || c == ':' || c == '.' {
                                        addr_input.push(c);
                                    }
                                }
                            }
                        }
                        menu_state.set_screen(MenuScreen::ClientAddr { addr_input, blink });
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn game_event(
        &mut self,
        event: winit::event::WindowEvent,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let AppState::InGame { game, dispatcher } = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(render_state) = &mut self.render_state {
                    if size.width > 0 && size.height > 0 {
                        render_state.config.width = size.width;
                        render_state.config.height = size.height;
                        render_state
                            .surface
                            .configure(&render_state.device, &render_state.config);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                block_on(game.net_state.update(dispatcher)).unwrap();
                game.update(crate::map::map_sdf);
                game.maybe_broadcast_update();
                if let Some(state) = &mut self.render_state {
                    state.render_game(game);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == winit::event::ElementState::Pressed;

                if let winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) =
                    event.physical_key
                {
                    game.keys.space = pressed;
                }

                if let winit::keyboard::Key::Character(s) = &event.logical_key {
                    match s.to_lowercase().as_str() {
                        "w" => game.keys.up = pressed,
                        "s" => game.keys.down = pressed,
                        "a" => game.keys.left = pressed,
                        "d" => game.keys.right = pressed,
                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                game.cursor_px = [position.x as f32, position.y as f32];
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(!focused);
                }
            }

            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create window once
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title(TITLE)
                            .with_inner_size(START_SIZE),
                    )
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());
            // Init wgpu using new window
            self.render_state = Some(block_on(RenderState::new(window)));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let action = match &self.state {
            AppState::MainMenu(_) => self.menu_event(event, event_loop),
            AppState::InGame { .. } => {
                self.game_event(event, event_loop);
                None
            }
        };
        if let Some(action) = action {
            self.transition_to(action);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
