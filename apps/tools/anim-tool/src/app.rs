use crate::{
    block_on,
    designer_state::DesignerState,
    render::RenderState,
    showcase_state::ShowcaseState,
    text::common::{
        Align, AtlasHandle, FontHandle, LineHeight, OutlineSpec, ShapeOptions, TextInstance,
        TextStyle, WaveSpec, WrapMode,
    },
};
use std::sync::Arc;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

const TITLE: &str = "Skeleton2D Designer Tool";
const START_SIZE: winit::dpi::LogicalSize<i32> = winit::dpi::LogicalSize::new(800, 600);
pub const SCRUB_TOL: f32 = 1. / 30.;
pub const FIXED_FPS: f32 = 60.;
pub const FIXED_DT: f32 = 1. / FIXED_FPS;
pub const CAM_KEY_SPEED: f32 = 0.5;

pub enum State {
    Designer(DesignerState),
    Showcase(ShowcaseState),
    Text,
}

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    state: State,
    // Text-mode input state.
    text_camera_keys: crate::common_state::CameraKeys,
    text_cursor_px: [f32; 2],
    text_scroll: f32,
    text_zoom: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            render_state: None,
            state: State::Text,
            text_camera_keys: crate::common_state::CameraKeys::default(),
            text_cursor_px: [0.0, 0.0],
            text_scroll: 0.0,
            text_zoom: 1.0,
        }
    }

    pub fn run(&mut self) -> Result<(), winit::error::EventLoopError> {
        winit::event_loop::EventLoop::new().unwrap().run_app(self)
    }

    pub fn update(&mut self) {
        match &mut self.state {
            State::Designer(state) => state.update(),
            State::Showcase(state) => state.update(),
            State::Text => {}
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
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Released
                    && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key =>
            {
                use winit::keyboard::KeyCode;
                match code {
                    KeyCode::Escape => {
                        event_loop.exit();
                        return;
                    }
                    KeyCode::F1 => {
                        match self.state {
                            State::Designer(_) => {
                                self.state = State::Showcase(ShowcaseState::new())
                            }
                            State::Showcase(_) => self.state = State::Text,
                            State::Text => self.state = State::Designer(DesignerState::default()),
                        }
                        return;
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Released
                    && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key =>
            {
                use winit::keyboard::KeyCode;
                if matches!(self.state, State::Text) {
                    match code {
                        KeyCode::KeyW => self.text_camera_keys.cam_up = false,
                        KeyCode::KeyS => self.text_camera_keys.cam_down = false,
                        KeyCode::KeyD => self.text_camera_keys.cam_right = false,
                        KeyCode::KeyA => self.text_camera_keys.cam_left = false,
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if matches!(self.state, State::Text) {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                    };
                    self.text_scroll = scroll;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if matches!(self.state, State::Text) {
                    self.text_cursor_px = [position.x as f32, position.y as f32];
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Pressed
                    && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key =>
            {
                use winit::keyboard::KeyCode;
                if matches!(self.state, State::Text) {
                    match code {
                        KeyCode::KeyW => self.text_camera_keys.cam_up = true,
                        KeyCode::KeyS => self.text_camera_keys.cam_down = true,
                        KeyCode::KeyD => self.text_camera_keys.cam_right = true,
                        KeyCode::KeyA => self.text_camera_keys.cam_left = true,
                        KeyCode::Equal => self.text_zoom *= 1.2,
                        KeyCode::Minus => self.text_zoom /= 1.2,
                        _ => {}
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(render_state) = &mut self.render_state {
                    render_state.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => match &mut self.state {
                State::Designer(designer_state) => {
                    if let Some(render_state) = &mut self.render_state {
                        render_state.render_designer(designer_state);
                    }
                }
                State::Showcase(showcase_state) => {
                    if let Some(render_state) = &mut self.render_state {
                        render_state.render_showcase(showcase_state.to_render([
                            render_state.config.width as f32,
                            render_state.config.height as f32,
                        ]));
                    }
                }
                State::Text => {
                    if let Some(render_state) = &mut self.render_state {
                        render_state.render_text(
                            &[
                                TextInstance::new(
                                    "Hello MTSDF",
                                    [-0.9, 0.],
                                    FontHandle::new(AtlasHandle::new(0), 0),
                                    ShapeOptions::new(
                                        0.05,
                                        None,
                                        WrapMode::PerWord,
                                        LineHeight::Metrics,
                                        Align::Left,
                                        0.,
                                        0.,
                                    ),
                                    TextStyle::new()
                                        .with_fill(Some([1., 1., 1., 1.]))
                                        .with_outline(Some(OutlineSpec::new(
                                            [1., 0., 0.5, 1.],
                                            3.,
                                        ))),
                                ),
                                //TODO: Add underline
                                TextInstance::new(
                                    "Text char wrapping test",
                                    [0.55, -0.3],
                                    FontHandle::new(AtlasHandle::new(0), 0),
                                    ShapeOptions::new(
                                        0.08,
                                        Some(0.45),
                                        WrapMode::PerChar,
                                        LineHeight::Metrics,
                                        Align::Left,
                                        0.,
                                        0.,
                                    ),
                                    TextStyle::new()
                                        .with_fill(Some([1., 0.5, 0.5, 1.]))
                                        .with_outline(Some(OutlineSpec::new([1., 0.5, 0., 1.], 3.)))
                                        .with_glyph_wave(Some(WaveSpec::new(0.005, 100., 0.))),
                                ),
                                // Add strikethough
                                TextInstance::new(
                                    "Text word wrapping test",
                                    [0.55, 0.3],
                                    FontHandle::new(AtlasHandle::new(0), 0),
                                    ShapeOptions::new(
                                        0.08,
                                        Some(0.45),
                                        WrapMode::PerWord,
                                        LineHeight::Metrics,
                                        Align::Left,
                                        0.,
                                        0.,
                                    ),
                                    TextStyle::new()
                                        .with_fill(Some([1., 0.5, 0.5, 1.]))
                                        .with_outline(Some(OutlineSpec::new([1., 0.5, 0., 1.], 3.)))
                                        .with_style_wave(Some(WaveSpec::new(0., 0., 0.))),
                                ),
                            ],
                            self.text_cursor_px,
                            self.text_zoom,
                        );
                    }
                }
            },
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
            State::Text => {}
        }
    }
}
