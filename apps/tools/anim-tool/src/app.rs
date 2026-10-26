use std::sync::Arc;

use crate::{
    block_on, designer_state::DesignerState, render::RenderState, showcase_state::ShowcaseState,
};
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

const TITLE: &str = "Skeleton2D Designer Tool";
const START_SIZE: winit::dpi::LogicalSize<i32> = winit::dpi::LogicalSize::new(800, 600);
pub const SCRUB_TOL: f32 = 1. / 30.;
pub const FPS: f32 = 60.;
pub const DT: f32 = 1. / FPS;
pub const CAM_KEY_SPEED: f32 = 0.5;

pub enum State {
    Designer(DesignerState),
    Showcase(ShowcaseState),
}

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    state: State,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            render_state: None,
            state: State::Designer(DesignerState::default()),
        }
    }

    pub fn run(&mut self) -> Result<(), winit::error::EventLoopError> {
        winit::event_loop::EventLoop::new().unwrap().run_app(self)
    }

    pub fn update(&mut self) {
        match &mut self.state {
            State::Designer(state) => state.update(),
            State::Showcase(state) => state.update(),
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

    fn about_to_wait(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        self.update();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::KeyboardInput { event, .. }
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
                    && code == winit::keyboard::KeyCode::Escape =>
            {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                if let Some(render_state) = &mut self.render_state {
                    render_state.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(render_state) = &mut self.render_state {
                    if let State::Designer(designer_state) = &mut self.state {
                        render_state.render_designer(designer_state);
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(!focused);
                }
            }
            _ => {}
        }
        match &mut self.state {
            State::Designer(state) => state.window_event(event),
            State::Showcase(state) => state.window_event(event),
        }
    }
}
