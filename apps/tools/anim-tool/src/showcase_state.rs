use crate::{
    app::FIXED_DT,
    camera::Camera,
    common_state::{CameraState, Globals, InputState, RenderData, Timeline},
};
use al_math::{sdf::Sdf2dPrim, vec::Vec2};

pub const SHOWCASE_SDF_COUNT: usize = 1;
pub const SHOWCASE_SDF_COLUMNS: usize = 1;
pub const SHOWCASE_SPACING: Vec2 = Vec2 { x: 2., y: 2. };

pub struct ShowcaseState {
    timeline: Timeline,
    camera_state: CameraState,
    keys: InputState,
    following: usize,
}

impl ShowcaseState {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(0.),
            camera_state: CameraState::new([800., 600.], Camera::new([0., 0.], 1.4)),
            keys: InputState::default(),
            following: 0,
        }
    }

    pub fn hud_string(&self) -> String {
        format!(
            "{} {:.3}/{:.3} loop {}",
            if self.timeline.playing() {
                "▶"
            } else {
                "▮▮"
            },
            self.timeline.time(),
            self.timeline.duration(),
            if self.timeline.looping() { "on" } else { "off" }
        )
    }

    pub fn update(&mut self) {
        self.camera_state
            .update(self.timeline.tick(), !self.keys.shift_held);
    }

    pub fn window_event(&mut self, event: winit::event::WindowEvent) {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.camera_state
                    .set_cursor_px([position.x as f32, position.y as f32]);

                if self.camera_state.panning() {
                    let c_px = self.camera_state.cursor_px();
                    let p_l = self.camera_state.pan_last();
                    let res = self.camera_state.resolution();
                    let dx = c_px.x - p_l.x;
                    let dy = c_px.y - p_l.y;
                    let h = res.y.max(1.0);
                    let cam = self.camera_state.camera_mut();
                    let vs = cam.view_size;
                    cam.pos.x -= (dx / h) * vs;
                    cam.pos.y += (dy / h) * vs;
                    self.camera_state.set_pan_last(c_px);
                }
            }

            WindowEvent::MouseInput { state, button, .. }
                if button == winit::event::MouseButton::Right =>
            {
                if state == winit::event::ElementState::Pressed {
                    self.camera_state.set_panning(true);
                    self.camera_state
                        .set_pan_last(self.camera_state.cursor_px());
                    self.camera_state.set_following(false);
                } else {
                    self.camera_state.set_panning(false);
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll = adjust radius of selected joint/bone or zoom if none selected.
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 40.0,
                };
                let cam = self.camera_state.camera_mut();
                cam.view_size = (cam.view_size * (1. - 0.1 * scroll)).clamp(0.2, 20.);
            }

            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.keys.ctrl_held = state.control_key();
                self.keys.shift_held = state.shift_key();
                self.keys.alt_held = state.alt_key();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::KeyCode;
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    let keys = self.camera_state.keys_mut();
                    match code {
                        KeyCode::KeyW => {
                            keys.cam_up = event.state == winit::event::ElementState::Pressed;
                        }
                        KeyCode::KeyS => {
                            keys.cam_down = event.state == winit::event::ElementState::Pressed
                        }
                        KeyCode::KeyA => {
                            keys.cam_left = event.state == winit::event::ElementState::Pressed
                        }
                        KeyCode::KeyD => {
                            keys.cam_right = event.state == winit::event::ElementState::Pressed
                        }
                        _ => {}
                    }

                    if event.state == winit::event::ElementState::Released {
                        match code {
                            KeyCode::Space => self.timeline.set_playing(!self.timeline.playing()),

                            KeyCode::Tab => {
                                if SHOWCASE_SDF_COUNT > 0 {
                                    if self.camera_state.following() {
                                        self.following = (self.following + 1) % SHOWCASE_SDF_COUNT;
                                    }
                                }
                                self.camera_state
                                    .set_target(Some(showcase_pos(self.following)));
                            }

                            KeyCode::ArrowLeft => self.timeline.scrub(-if self.keys.alt_held {
                                FIXED_DT
                            } else {
                                5. * FIXED_DT
                            }),

                            KeyCode::ArrowRight => self.timeline.scrub(if self.keys.alt_held {
                                FIXED_DT
                            } else {
                                5. * FIXED_DT
                            }),

                            KeyCode::Home => self.timeline.set_time(0.),

                            KeyCode::KeyL => self.timeline.set_looping(!self.timeline.looping()),

                            KeyCode::KeyF => self.following = !self.following,

                            _ => {}
                        }
                    }
                }
            }

            _ => {}
        }
    }

    pub fn to_render(&mut self, resolution: impl Into<Vec2>) -> RenderData {
        let resolution = resolution.into();
        self.camera_state.set_resolution(resolution);

        let cam = self.camera_state.camera();
        let c_px = self.camera_state.cursor_px();
        let cx = c_px.x - 0.5 * resolution.x;
        let cy = c_px.y - 0.5 * resolution.y;
        let c_world = cam.pos
            + [
                cx / resolution.y * cam.view_size,
                -cy / resolution.y * cam.view_size,
            ];
        self.camera_state.set_cursor_world(c_world);

        let offset = Vec2::ZERO;
        let radius = 0.5;
        let m_inv = al_math::matrix::Mat2::IDENTITY.to_cols_array();
        let color = [0.9, 0.9, 0.9, 1.];
        let mut sdfs = Vec::new();

        let mut pos = 0;
        sdfs.push(Sdf2dPrim::circle(showcase_pos(pos) + offset, radius, m_inv, color));
        pos += 1;
        sdfs.push(Sdf2dPrim::circle(showcase_pos(pos) + offset, radius, m_inv, color));

        // Cursor
        sdfs.push(Sdf2dPrim::circle(c_world, 0.006, m_inv, [0.4, 1., 0.6, 1.]));

        RenderData::new(
            Globals::new(resolution, cam.pos, c_world, cam.view_size),
            sdfs,
            self.hud_string(),
        )
    }
}

fn showcase_pos(following: usize) -> Vec2 {
    [
        (following / SHOWCASE_SDF_COLUMNS) as f32,
        (following % SHOWCASE_SDF_COLUMNS) as f32,
    ] * SHOWCASE_SPACING
}
