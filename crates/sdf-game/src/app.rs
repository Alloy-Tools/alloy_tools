use crate::{
    block_on,
    game::{GameState, HealthUpdate, PlayerUpdate, ProjectileSpawn, Scoreboard},
    net::{Dispatcher, NetEvent, NetworkState},
    render::RenderState,
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler, dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop,
    window::Window,
};

const TITLE: &str = "SDF Game MVP";
const START_SIZE: LogicalSize<i32> = LogicalSize::new(800, 600);

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<RenderState>,
    game: GameState,
    dispatcher: Dispatcher,
}

impl App {
    pub async fn new(
        net_state: NetworkState,
        send_interval: std::time::Duration,
        any: crate::net::AnyStore,
        keyed: crate::net::KeyedStore,
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
            .register_event(|_, state, evt: NetEvent| {
                Box::pin(async move {
                    match evt {
                        NetEvent::NoOp => {}
                        NetEvent::Connected(id) => state.write().await.set_local_conn(Some(id)),
                        NetEvent::Disconnected(id) => state.write().await.remove_player(id),
                        NetEvent::Welcome(id) => state.write().await.set_server_id(Some(id)),
                    }
                })
            })
            .await
            .unwrap();

        Self {
            window: None,
            state: None,
            game: GameState::new(send_interval, net_state),
            dispatcher,
        }
    }

    pub fn run(&mut self) -> Result<(), winit::error::EventLoopError> {
        EventLoop::new().unwrap().run_app(self)
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
            self.state = Some(block_on(RenderState::new(window)));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    if size.width > 0 && size.height > 0 {
                        state.config.width = size.width;
                        state.config.height = size.height;
                        state.surface.configure(&state.device, &state.config);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                block_on(self.game.net_state.update(&self.dispatcher)).unwrap();
                self.game.update(crate::map::map_sdf);
                self.game.maybe_broadcast_update();
                if let Some(state) = &self.state {
                    state.render(&mut self.game);
                }
                if let Some(window) = &self.window {
                    window.set_title(&format_scorboard(TITLE, &self.game));
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == winit::event::ElementState::Pressed;

                if let winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) =
                    event.physical_key
                {
                    self.game.keys.space = pressed;
                }

                if let winit::keyboard::Key::Character(s) = &event.logical_key {
                    match s.to_lowercase().as_str() {
                        "w" => self.game.keys.up = pressed,
                        "s" => self.game.keys.down = pressed,
                        "a" => self.game.keys.left = pressed,
                        "d" => self.game.keys.right = pressed,
                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.game.cursor_px = [position.x as f32, position.y as f32];
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(!focused);
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn format_scorboard(base: &str, game: &GameState) -> String {
    if game.scoreboard.entries().is_empty() {
        return base.to_string();
    }
    let mut parts = game
        .scoreboard
        .entries()
        .iter()
        .map(|e| format!("c{} {}/{}", e.conn_id, e.kills, e.deaths))
        .collect::<Vec<_>>();
    parts.sort();
    format!("{} | {}", base, parts.join(" - "))
}
