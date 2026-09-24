use crate::{
    block_on,
    game::{GameState, PlayerUpdate},
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

        // Connection lifecycle
        dispatcher
            .register_event(|_, state, evt: NetEvent| {
                Box::pin(async move {
                    match evt {
                        NetEvent::NoOp => {}
                        NetEvent::Connected(id) => state.write().await.set_local_conn(Some(id)),
                        NetEvent::Disconnected(id) => {
                            let mut s = state.write().await;
                            if s.local_conn() == Some(id) {
                                s.set_local_conn(None);
                            }
                            s.remove_player(id);
                        }
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
                self.game.update();
                self.game.maybe_broadcast_update();
                if let Some(state) = &self.state {
                    state.render(&self.game);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == winit::event::ElementState::Pressed;
                if let winit::keyboard::Key::Character(s) = &event.logical_key {
                    let s = s.as_str().to_lowercase();
                    if s == "w" {
                        self.game.keys.up = pressed;
                    } else if s == "s" {
                        self.game.keys.down = pressed;
                    } else if s == "a" {
                        self.game.keys.left = pressed;
                    } else if s == "d" {
                        self.game.keys.right = pressed;
                    }
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
