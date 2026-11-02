use crate::{
    app::FIXED_DT,
    camera::Camera,
    common_state::{CameraState, Globals, InputState, RenderData, Timeline},
};
use al_math::{
    matrix::Mat2,
    sdf::{Sdf2dBoolean, Sdf2dGroup, Sdf2dOp, Sdf2dPrim},
    vec::Vec2,
};

pub const SHOWCASE_SET_COUNT: usize = 3;
pub const SHOWCASE_SET_NORMAL: usize = 0;
pub const SHOWCASE_SET_INFINITE: usize = 1;
pub const SHOWCASE_SET_GROUPS: usize = 2;

pub const SHOWCASE_SDF_COUNTS: [usize; SHOWCASE_SET_COUNT] = [36, 4, 8];
pub const SHOWCASE_SDF_COLUMNS: [usize; SHOWCASE_SET_COUNT] = [7, 4, 4];
pub const SHOWCASE_SPACING: Vec2 = Vec2 { x: 2., y: 2. };

const OP_STEP: f32 = 0.02;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BackgroundMode {
    #[default]
    Grid,
    Solid,
    Bands,
}

impl BackgroundMode {
    fn next(self) -> Self {
        match self {
            Self::Grid => Self::Solid,
            Self::Solid => Self::Bands,
            Self::Bands => Self::Grid,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Solid => "solid",
            Self::Bands => "bands",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OpMode {
    Onion,
    #[default]
    Round,
    Elongate,
}

impl OpMode {
    fn label(self) -> &'static str {
        match self {
            Self::Onion => "onion",
            Self::Round => "round",
            Self::Elongate => "elongate",
        }
    }
}

pub struct ShowcaseState {
    timeline: Timeline,
    camera_state: CameraState,
    keys: InputState,
    following: usize,
    curr_set: usize,
    bg_mode: BackgroundMode,
    op_mode: OpMode,
    op_onion_delta: f32,
    op_round_delta: f32,
    op_elongate_x_delta: f32,
    op_elongate_y_delta: f32,
    scale: Vec2,
    rotation: f32,
}

impl ShowcaseState {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(0.),
            camera_state: CameraState::new([800., 600.], Camera::new([0., 0.], 1.4)),
            keys: InputState::default(),
            following: 0,
            curr_set: 0,
            bg_mode: BackgroundMode::default(),
            op_mode: OpMode::default(),
            op_onion_delta: 0.,
            op_round_delta: 0.,
            op_elongate_x_delta: 0.,
            op_elongate_y_delta: 0.,
            scale: Vec2::ONE,
            rotation: 0.,
        }
    }

    pub fn hud_string(&self) -> String {
        format!(
            "{} {:.3}/{:.3} loop {} | bg {} | op {} on {:.2} ro {:.2} el x {:.2} y {:.2}",
            if self.timeline.playing() {
                "▶"
            } else {
                "▮▮"
            },
            self.timeline.time(),
            self.timeline.duration(),
            if self.timeline.looping() { "on" } else { "off" },
            self.bg_mode.label(),
            self.op_mode.label(),
            self.op_onion_delta,
            self.op_round_delta,
            self.op_elongate_x_delta,
            self.op_elongate_y_delta,
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
                                let count = SHOWCASE_SDF_COUNTS[self.curr_set];
                                if count > 0 {
                                    self.following %= count;
                                    if self.camera_state.following() {
                                        self.following = (self.following + 1) % count;
                                    }
                                }
                                self.camera_state
                                    .set_target(Some(showcase_pos(self.curr_set, self.following)));
                            }

                            KeyCode::KeyT => {
                                self.curr_set = (self.curr_set + 1) % SHOWCASE_SET_COUNT;
                                self.following = 0;
                                self.camera_state
                                    .set_target(Some(showcase_pos(self.curr_set, 0)));
                            }

                            KeyCode::ArrowLeft => {
                                if self.keys.ctrl_held && self.keys.shift_held {
                                    self.rotation += 0.1;
                                } else if self.keys.ctrl_held {
                                    self.nudge_elongate_x(-OP_STEP);
                                } else {
                                    self.timeline.scrub(-if self.keys.alt_held {
                                        FIXED_DT
                                    } else {
                                        5. * FIXED_DT
                                    })
                                }
                            }

                            KeyCode::ArrowRight => {
                                if self.keys.ctrl_held && self.keys.shift_held {
                                    self.rotation -= 0.1;
                                } else if self.keys.ctrl_held {
                                    self.nudge_elongate_x(OP_STEP);
                                } else {
                                    self.timeline.scrub(if self.keys.alt_held {
                                        FIXED_DT
                                    } else {
                                        5. * FIXED_DT
                                    })
                                }
                            }

                            KeyCode::ArrowUp => {
                                if self.keys.ctrl_held {
                                    self.nudge_op(OP_STEP);
                                } else {
                                    self.scale += Vec2::from(0.1);
                                }
                            }

                            KeyCode::ArrowDown => {
                                if self.keys.ctrl_held {
                                    self.nudge_op(-OP_STEP);
                                } else {
                                    self.scale -= Vec2::from(0.1);
                                }
                            }

                            KeyCode::Home => self.timeline.set_time(0.),

                            KeyCode::KeyP => self.timeline.set_looping(!self.timeline.looping()),

                            KeyCode::KeyF => self.following = !self.following,

                            KeyCode::KeyB => self.bg_mode = self.bg_mode.next(),

                            KeyCode::KeyO => self.op_mode = OpMode::Onion,
                            KeyCode::KeyR => self.op_mode = OpMode::Round,
                            KeyCode::KeyL => self.op_mode = OpMode::Elongate,

                            KeyCode::KeyQ => {
                                self.scale = Vec2::new(
                                    (self.scale.x - 0.05).max(0.2),
                                    (self.scale.y + 0.05).min(3.0),
                                );
                            }
                            KeyCode::KeyE => {
                                self.scale = Vec2::new(
                                    (self.scale.x + 0.05).min(3.0),
                                    (self.scale.y - 0.05).max(0.2),
                                );
                            }

                            _ => {}
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn nudge_op(&mut self, delta: f32) {
        match self.op_mode {
            OpMode::Onion => {
                self.op_onion_delta = (self.op_onion_delta + delta).clamp(0.0, 0.5);
            }
            OpMode::Round => {
                self.op_round_delta = (self.op_round_delta + delta).clamp(-0.5, 0.5);
            }
            OpMode::Elongate => {
                self.op_elongate_y_delta = (self.op_elongate_y_delta + delta).clamp(0.0, 0.5);
            }
        }
    }

    fn nudge_elongate_x(&mut self, delta: f32) {
        if self.op_mode == OpMode::Elongate {
            self.op_elongate_x_delta = (self.op_elongate_x_delta + delta).clamp(0.0, 0.5);
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

        let radius = 0.5;
        let m_inv = (Mat2::from_angle(self.rotation) * Mat2::from_scale(self.scale))
            .try_inverse()
            .unwrap_or(Mat2::IDENTITY)
            .to_cols_array();
        let color = [0.9, 0.9, 0.9, 1.];
        let mut sdfs = Vec::new();

        if self.curr_set == SHOWCASE_SET_NORMAL {
            let mut pos = 0;
            sdfs.push(Sdf2dPrim::circle(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                radius,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::capsule(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                radius,
                radius,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::rect(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                radius,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::ellipse(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.5, 0.3],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::uneven_capsule(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.7,
                0.28,
                0.1,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::rounded_rect(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.42, 0.3],
                [0.18, 0.06, 0.12, 0.24].into(),
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::chamfer_rect(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.42, 0.3],
                [0.1, 0.16, 0.2, 0.08].into(),
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::rhombus(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.5, 0.3],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::trapezoid(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                0.25,
                0.4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::parallelogram(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.45,
                0.3,
                0.22,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::triangle(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [-0.25, -0.15],
                [0.65, -0.55],
                [0.0, 0.45],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::equ_triangle(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::iso_triangle(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.65,
                0.42,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::pentagon(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.48,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::hexagon(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.48,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::octogon(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.48,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::pentagram(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.45,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::hexagram(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::star(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.45,
                3.0,
                4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::pie(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.5, 0.8660254],
                0.5,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::cut_disk(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                0.18,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::arc(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.5, 0.8660254],
                0.38,
                0.08,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::ring(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.70710677, 0.70710677],
                0.42,
                0.12,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::horseshoe(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.5, 0.8660254],
                0.38,
                [0.12, 0.08],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::vesica(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                0.4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::moon(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.25,
                0.45,
                0.38,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::egg(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                0.2,
                0.3,
                0.8,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::heart(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::rounded_x(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                0.08,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::cross(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.45, 0.18],
                0.08,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::circle_cross(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.5,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::blobby_cross(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                0.4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::quadratic_bezier(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [-0.45, -0.3],
                [0.0, 0.5],
                [0.45, -0.3],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::quadratic_circle(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::tunnel(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.4, 0.3],
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::stairs(
                showcase_pos(SHOWCASE_SET_NORMAL, pos),
                [0.18, 0.18],
                3.0,
                m_inv,
                color,
            ));
        } else if self.curr_set == SHOWCASE_SET_INFINITE {
            let mut pos = 0;
            sdfs.push(Sdf2dPrim::parabola(
                showcase_pos(SHOWCASE_SET_INFINITE, pos),
                1.5,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::parabola_segment(
                showcase_pos(SHOWCASE_SET_INFINITE, pos),
                0.5,
                0.4,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::hyperbola(
                showcase_pos(SHOWCASE_SET_INFINITE, pos),
                0.4,
                0.5,
                m_inv,
                color,
            ));
            pos += 1;
            sdfs.push(Sdf2dPrim::circle_wave(
                showcase_pos(SHOWCASE_SET_INFINITE, pos),
                0.5,
                0.35,
                m_inv,
                color,
            ));
        } else if self.curr_set == SHOWCASE_SET_GROUPS {
            // Group 0: Circle & capsule
            sdfs.push(Sdf2dPrim::circle(
                Vec2::new(-1.75, 1.),
                0.5,
                m_inv,
                color,
            ));
            sdfs.push(Sdf2dPrim::capsule(
                Vec2::new(-1.5, 1.),
                0.8,
                0.25,
                m_inv,
                color,
            ));

            // Group 1: Rect & circle
            sdfs.push(Sdf2dPrim::rect(
                Vec2::new(1.5, 1.),
                [0.45, 0.45],
                m_inv,
                color,
            ));
            sdfs.push(Sdf2dPrim::circle(
                Vec2::new(1.5, 1.),
                0.35,
                m_inv,
                color,
            ));

            // Group 2: rect & rect
            sdfs.push(Sdf2dPrim::rect(
                Vec2::new(-1.75, -1.),
                [0.25, 0.5],
                m_inv,
                color,
            ));
            sdfs.push(Sdf2dPrim::rect(
                Vec2::new(-1.25, -1.),
                [0.5, 0.25],
                m_inv,
                color,
            ));

            // Group 3: Star & circle
            sdfs.push(Sdf2dPrim::star(
                Vec2::new(1.25, -1.),
                0.45,
                2.5,
                5,
                m_inv,
                color,
            ));
            sdfs.push(Sdf2dPrim::circle(
                Vec2::new(1.75, -1.),
                0.35,
                m_inv,
                color,
            ));
        }

        debug_assert_eq!(
            sdfs.len(),
            SHOWCASE_SDF_COUNTS[self.curr_set],
            "showcase shape list and SHOWCASE_SDF_COUNTS disagree"
        );

        // Cursor
        sdfs.push(Sdf2dPrim::circle(c_world, 0.006, m_inv, [0.4, 1., 0.6, 1.]));

        let mut ops: Vec<Sdf2dOp> = Vec::with_capacity(sdfs.len());

        if self.curr_set == SHOWCASE_SET_GROUPS {
            // circle
            ops.push(Sdf2dOp::default());
            // capsule
            ops.push(Sdf2dOp::default());

            // rect
            ops.push(Sdf2dOp::default());
            // circle
            ops.push(Sdf2dOp::default().with_boolean(Sdf2dBoolean::Subtraction));

            // rect
            ops.push(Sdf2dOp::default());
            // rect
            ops.push(Sdf2dOp::default().with_boolean(Sdf2dBoolean::SmoothUnion(0.1)));

            // star
            ops.push(Sdf2dOp::default());
            // circle
            ops.push(Sdf2dOp::default().with_boolean(Sdf2dBoolean::SmoothSubtraction(0.2)));
        } else {
            ops = vec![Sdf2dOp::default(); sdfs.len()];
        }

        // Ctrl+Up/Down nudges the selected field on every shape
        let shape_count = ops.len().saturating_sub(1);
        for op in &mut ops[..shape_count] {
            op.set_onion((op.onion() + self.op_onion_delta).max(0.0));
            op.set_round(op.round() + self.op_round_delta);
            let elongate =
                op.elongate() + Vec2::new(self.op_elongate_x_delta, self.op_elongate_y_delta);
            op.set_elongate(Vec2::new(elongate.x.max(0.0), elongate.y.max(0.0)));
        }

        // Build groups for the groups set.
        let groups = if self.curr_set == SHOWCASE_SET_GROUPS {
            vec![
                Sdf2dGroup::new(0, 2), // circle + capsule union
                Sdf2dGroup::new(2, 2), // rect - circle subtraction
                Sdf2dGroup::new(4, 2), // circle + circle smooth union
                Sdf2dGroup::new(6, 2), // star - circle smooth subtraction
            ]
        } else {
            Vec::new()
        };

        let mut globals = Globals::new(resolution, cam.pos, c_world, cam.view_size);
        globals.aux = match self.bg_mode {
            BackgroundMode::Grid => 0.,
            BackgroundMode::Solid => 1.,
            BackgroundMode::Bands => 20.,
        };
        RenderData::new(globals, sdfs, ops, groups, self.hud_string())
    }
}

fn set_count(set: usize) -> usize {
    SHOWCASE_SDF_COUNTS[set]
}

fn set_columns(set: usize) -> usize {
    SHOWCASE_SDF_COLUMNS[set]
}

fn set_offset(set: usize) -> Vec2 {
    let count = set_count(set).max(1);
    let columns = set_columns(set).min(count).max(1);
    let rows = (count + columns - 1) / columns;
    Vec2 {
        x: -0.5 * (columns as f32 - 1.) * SHOWCASE_SPACING.x,
        y: 0.5 * (rows as f32 - 1.) * SHOWCASE_SPACING.y,
    }
}

fn showcase_pos(set: usize, following: usize) -> Vec2 {
    let columns = set_columns(set);
    [
        (following % columns) as f32,
        -((following / columns) as f32),
    ] * SHOWCASE_SPACING
        + set_offset(set)
}
