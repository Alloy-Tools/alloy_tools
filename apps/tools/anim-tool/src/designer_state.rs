use crate::{
    app::{FIXED_DT, SCRUB_TOL},
    camera::Camera,
    common_state::InputState,
};
use al_anim::{
    anim_clip::{AnimationClip2d, AttachType},
    vfx::Vfx2d,
    Easing,
};
use al_math::{
    sdf::{Sdf2d, Sdf2dKind},
    transform::Transform2d,
    vec::{Vec2, Vec4},
};
use al_skeleton::{
    collider_kind::ColliderKind,
    pose::Pose2d,
    skeleton::{ColliderRef, Skeleton2DBuilder, Skeleton2d},
};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditMode {
    Pose,
    Rest,
}

impl std::fmt::Display for EditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditMode::Pose => write!(f, "Pose"),
            EditMode::Rest => write!(f, "Rest"),
        }
    }
}

impl EditMode {
    pub fn cycle(&mut self) {
        *self = match self {
            EditMode::Pose => EditMode::Rest,
            EditMode::Rest => EditMode::Pose,
        }
    }

    pub fn status(&self) -> String {
        match self {
            EditMode::Pose => "Editing 'Pose' (rotations)".into(),
            EditMode::Rest => "Editing 'Rest' (translations)".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragMode {
    MoveJoint,
    RotateJoint,
}

#[derive(Clone, Debug)]
pub enum Snapshot {
    Skeleton(Skeleton2d),
    Pose(Pose2d),
    Animation(Vec<AnimationClip2d>, usize),
}

pub struct History {
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn record(&mut self, snapshot: Snapshot) {
        self.undo.push(snapshot);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum OnionMode {
    #[default]
    Keys,
    Time,
}

impl OnionMode {
    pub fn cycle(self) -> Self {
        match self {
            OnionMode::Keys => OnionMode::Time,
            OnionMode::Time => OnionMode::Keys,
        }
    }
}

pub struct DesignerState {
    pub skeleton: Skeleton2d,
    pub pose: Pose2d,
    pub easing: Easing,

    pub selected: Option<ColliderRef>,
    pub drag: Option<DragMode>,
    pub edit_mode: EditMode,
    pub show_vfx: bool,
    pub show_extras: bool,
    pub extra_active: Vec<bool>,
    pub onion_keys: usize,
    pub onion_mode: OnionMode,

    pub clips: Vec<AnimationClip2d>,
    pub current_clip: usize,
    pub time: f32,
    pub playing: bool,
    pub last_update: Instant,

    pub camera: Camera,
    pub following: bool,
    pub panning: bool,
    pub pan_last: [f32; 2],
    pub cursor_px: [f32; 2],
    pub cursor_world: al_math::vec::Vec2,
    pub resolution: [f32; 2],

    pub history: History,
    pub last_scroll: Instant,
    pub scroll_dirty: bool,
    pub drag_dirty: bool,
    pub keys: InputState,

    pub status: String,
}

impl Default for DesignerState {
    fn default() -> Self {
        let hurt = ColliderKind::HURT;
        let none = ColliderKind::NONE;
        let mut builder = Skeleton2DBuilder::new();
        // ----- spine -----
        let hips = builder.joint("hips", None, Vec2::new(0., -0.05), 0.075, 0., hurt, 0.);
        let chest = builder.joint(
            "chest",
            Some(hips),
            Vec2::new(0.02, 0.20),
            0.085,
            0.,
            hurt,
            0.,
        );

        let neck = builder.joint("neck", Some(chest), Vec2::new(0.01, 0.14), 0., 0., none, 0.);
        let head = builder.joint(
            "head",
            Some(neck),
            Vec2::new(0.02, 0.05),
            0.09,
            0.,
            hurt,
            0.,
        );

        builder.bone("torso", hips, chest, 0.09, hurt, 0.);
        builder.bone("neck", chest, head, 0.045, hurt, 0.);

        // ----- arms -----
        let sh_f = builder.joint(
            "shoulder_f",
            Some(chest),
            Vec2::new(0.07, 0.03),
            0.,
            0.,
            none,
            1.2,
        );
        let el_f = builder.joint(
            "elbow_f",
            Some(sh_f),
            Vec2::new(0.1, -0.12),
            0.,
            0.,
            none,
            1.2,
        );
        let hd_f = builder.joint(
            "hand_f",
            Some(el_f),
            Vec2::new(0.1, -0.1),
            0.045,
            0.,
            hurt,
            1.2,
        );

        builder.bone("upper_arm_f", sh_f, el_f, 0.045, hurt, 1.2);
        builder.bone("forearm_f", el_f, hd_f, 0.04, hurt, 1.2);

        let sh_b = builder.joint(
            "shoulder_b",
            Some(chest),
            Vec2::new(-0.05, 0.03),
            0.,
            0.,
            none,
            -1.2,
        );
        let el_b = builder.joint(
            "elbow_b",
            Some(sh_b),
            Vec2::new(-0.1, -0.14),
            0.,
            0.,
            none,
            -1.2,
        );
        let hd_b = builder.joint(
            "hand_b",
            Some(el_b),
            Vec2::new(0.07, -0.08),
            0.045,
            0.,
            hurt,
            -1.2,
        );

        builder.bone("upper_arm_b", sh_b, el_b, 0.045, hurt, -1.2);
        builder.bone("forearm_b", el_b, hd_b, 0.04, hurt, -1.2);

        // ----- legs -----
        let kn_f = builder.joint(
            "knee_f",
            Some(hips),
            Vec2::new(0.08, -0.22),
            0.,
            0.,
            hurt,
            0.6,
        );
        let ft_f = builder.joint(
            "foot_f",
            Some(kn_f),
            Vec2::new(0.04, -0.22),
            0.055,
            0.,
            hurt,
            0.6,
        );

        builder.bone("thigh_f", hips, kn_f, 0.06, hurt, 0.6);
        builder.bone("shin_f", kn_f, ft_f, 0.05, hurt, 0.6);

        let kn_b = builder.joint(
            "knee_b",
            Some(hips),
            Vec2::new(-0.06, -0.22),
            0.,
            0.,
            hurt,
            -0.6,
        );
        let ft_b = builder.joint(
            "foot_b",
            Some(kn_b),
            Vec2::new(-0.06, -0.22),
            0.055,
            0.,
            hurt,
            -0.6,
        );

        builder.bone("thigh_b", hips, kn_b, 0.06, hurt, -0.6);
        builder.bone("shin_b", kn_b, ft_b, 0.05, hurt, -0.6);

        let _e1 = builder.extra(
            "blade_front",
            Some(hd_f),
            Sdf2d::new(
                Transform2d::from_translation(Vec2::new(0.02, 0.)),
                Sdf2dKind::capsule(0.1, 0.015),
            ),
            ColliderKind::HIT,
        );

        let mut vfx = Vec::new();
        // Explosion
        let explosion_start = 0.2;
        let explosion_offset = Vec2::new(1., 0.);
        // White flash
        vfx.push(Vfx2d::new(
            "explosion.flash",
            explosion_start,
            0.08,
            AttachType::From {
                joint: hd_f,
                offset: explosion_offset,
            },
            Sdf2d::identity(Sdf2dKind::circle(0.07)),
            (0.4.into(), 1.6.into()),
            (1., 0.),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(1., 1., 0.9, 1.),
            Easing::EaseOut,
        ));
        // Orange core
        vfx.push(Vfx2d::new(
            "explosion.core",
            explosion_start,
            0.25,
            AttachType::From {
                joint: hd_f,
                offset: explosion_offset,
            },
            Sdf2d::identity(Sdf2dKind::circle(0.1)),
            (0.5.into(), 2.2.into()),
            (0.95, 0.),
            (0., 0.),
            Vec2::new(0., 0.01),
            Vec4::new(1., 0.55, 0.15, 1.),
            Easing::EaseOut,
        ));
        // Smoke
        vfx.push(Vfx2d::new(
            "explosion.Smoke",
            explosion_start + 0.03,
            0.55,
            AttachType::From {
                joint: hd_f,
                offset: explosion_offset,
            },
            Sdf2d::identity(Sdf2dKind::circle(0.13)),
            (0.8.into(), 3.2.into()),
            (0.55, 0.),
            (0., 0.),
            Vec2::new(-0.1, 0.04),
            Vec4::new(0.2, 0.16, 0.14, 1.),
            Easing::EaseOut,
        ));
        // Ring
        vfx.push(Vfx2d::new(
            "explosion.ring",
            explosion_start,
            0.15,
            AttachType::From {
                joint: hd_f,
                offset: explosion_offset,
            },
            Sdf2d::new(
                Transform2d::from_translation(Vec2::new(-0.02, 0.)),
                Sdf2dKind::capsule(0.02, 0.004),
            ),
            ([0.5, 0.5].into(), [14., 1.].into()),
            (1., 0.),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(1., 0.85, 0.4, 1.),
            Easing::EaseOut,
        ));

        /*// Laser
        let laser_start = 0.15;
        let laser_len = 0.32;//10.;
        // Charge
        vfx.push(Vfx2d::new(
            "laser.charge",
            laser_start - 0.1,
            0.12,
            AttachType::Follow {
                joint: hd_f,
                offset: Vec2::ZERO,
            },
            Sdf2d::circle(
                Vec2::ZERO,
                0.03,
            ),
            (0.2.into(), 1.4.into()),
            (0.3, 1.),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(0.5, 0.9, 1., 1.),
            Easing::EaseIn,
        ));
        // Core
        vfx.push(Vfx2d::new(
            "laser.core",
            laser_start,
            0.28,
            AttachType::Follow {
                joint: hd_f,
                offset: Vec2::ZERO,
            },
            Sdf2d::capsule(
                Vec2::ZERO,
                Vec2::new(laser_len, 0.),
                0.005,
            ),
            (0.05.into(), 1.0.into()),
            (1., 0.4),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(1., 1., 1., 1.),
            Easing::EaseOut,
        ));
        // Glow
        vfx.push(Vfx2d::new(
            "laser.glow",
            laser_start,
            0.28,
            AttachType::Follow {
                joint: hd_f,
                offset: Vec2::ZERO,
            },
            Sdf2d::capsule(
                Vec2::ZERO,
                Vec2::new(laser_len, 0.),
                0.015,
            ),
            ([0.05, 1.].into(), [1., 1.].into()),
            (0.7, 0.2),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(0.3, 0.75, 1., 1.),
            Easing::EaseOut,
        ));
        // Flash
        vfx.push(Vfx2d::new(
            "laser.flash",
            laser_start,
            0.07,
            AttachType::Follow {
                joint: hd_f,
                offset: Vec2::ZERO,
            },
            Sdf2d::circle(
                Vec2::ZERO,
                0.045,
            ),
            (0.3.into(), 1.5.into()),
            (1., 0.),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(0.9, 1., 1., 1.),
            Easing::EaseOut,
        ));
        // Impact
        vfx.push(Vfx2d::new(
            "laser.impact",
            laser_start + 0.02,
            0.16,
            AttachType::Follow {
                joint: hd_f,
                offset: Vec2::new(laser_len, 0.),
            },
            Sdf2d::circle(
                Vec2::ZERO,
                0.05,
            ),
            (0.4.into(), 1.8.into()),
            (1., 0.),
            (0., 0.),
            Vec2::ZERO,
            Vec4::new(0.6, 0.9, 1., 1.),
            Easing::EaseOut,
        ));*/

        let skeleton = builder.build();
        if let Err(e) = skeleton.validate() {
            panic!("Skeleton validation failed: {e}")
        }
        Self::new(skeleton, &vfx)
    }
}

impl DesignerState {
    pub fn new(skeleton: Skeleton2d, vfx: &[Vfx2d]) -> Self {
        let extra_len = skeleton.extras().len();
        let pose = Pose2d::rest(skeleton.joints().len());
        let mut default_clip = AnimationClip2d::new("default");
        default_clip.vfx_mut().extend_from_slice(vfx);
        Self {
            skeleton,
            pose,
            easing: Easing::Linear,
            selected: None,
            drag: None,
            edit_mode: EditMode::Pose,
            show_vfx: true,
            show_extras: false,
            extra_active: vec![false; extra_len],
            onion_keys: 0,
            onion_mode: Default::default(),
            clips: vec![default_clip],
            current_clip: 0,
            time: 0.,
            playing: false,
            last_update: Instant::now(),
            camera: Camera::new([0., 0.], 1.4),
            following: true,
            panning: false,
            pan_last: [0., 0.],
            cursor_px: [0., 0.],
            cursor_world: al_math::vec::Vec2::ZERO,
            resolution: [800., 600.],
            history: History::new(),
            last_scroll: Instant::now(),
            scroll_dirty: false,
            drag_dirty: false,
            keys: InputState::default(),
            status: "Ready.".into(),
        }
    }

    /// Nearest joint origin within ~2% of view height.
    pub fn pick(&self) -> Option<ColliderRef> {
        let world = self.skeleton.world_transforms(&self.pose);
        let pick_r = 0.02 * self.camera.view_size; // ~2% of view height
        let mut best: Option<(ColliderRef, f32)> = None;
        for (i, _) in self.skeleton.joints().iter().enumerate() {
            let c = world[i].transform_point(al_math::vec::Vec2::ZERO);
            let d = (c - self.cursor_world).length();
            if d <= pick_r && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((ColliderRef::Joint(i), d));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn record(&mut self, snapshot: Snapshot) {
        self.history.record(snapshot);
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.history.undo.pop() {
            match snapshot {
                Snapshot::Skeleton(skeleton) => {
                    self.history
                        .redo
                        .push(Snapshot::Skeleton(std::mem::take(&mut self.skeleton)));
                    self.skeleton = skeleton;
                }
                Snapshot::Pose(pose) => {
                    self.history
                        .redo
                        .push(Snapshot::Pose(std::mem::take(&mut self.pose)));
                    self.pose = pose;
                }
                Snapshot::Animation(clips, current_clip) => {
                    self.history.redo.push(Snapshot::Animation(
                        std::mem::take(&mut self.clips),
                        self.current_clip,
                    ));
                    self.clips = clips;
                    self.current_clip = current_clip;
                }
            }
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.history.redo.pop() {
            match snapshot {
                Snapshot::Skeleton(skeleton) => {
                    self.history
                        .undo
                        .push(Snapshot::Skeleton(std::mem::take(&mut self.skeleton)));
                    self.skeleton = skeleton;
                }
                Snapshot::Pose(pose) => {
                    self.history
                        .undo
                        .push(Snapshot::Pose(std::mem::take(&mut self.pose)));
                    self.pose = pose;
                }
                Snapshot::Animation(clips, current_clip) => {
                    self.history.undo.push(Snapshot::Animation(
                        std::mem::take(&mut self.clips),
                        self.current_clip,
                    ));
                    self.clips = clips;
                    self.current_clip = current_clip;
                }
            }
            return true;
        }
        false
    }

    pub fn clip(&self) -> &AnimationClip2d {
        &self.clips[self.current_clip]
    }

    pub fn clip_mut(&mut self) -> &mut AnimationClip2d {
        &mut self.clips[self.current_clip]
    }

    pub fn with_clip<R>(&mut self, f: impl FnOnce(&mut AnimationClip2d) -> R) -> R {
        f(&mut self.clips[self.current_clip])
    }

    pub fn cycle_clip(&mut self, delta: isize) {
        if self.clips.is_empty() {
            return;
        }
        let n = self.clips.len() as isize;
        self.current_clip = (self.current_clip as isize + delta).rem_euclid(n) as usize;
        self.time = 0.;
        self.playing = false;
        if !self.clip().keyframes().is_empty() {
            self.pose = self.clip().sample(0.);
        }
    }

    pub fn hud_string(&self) -> String {
        let body_line = format!(
            "{} joints, {} bones | [Tab] cycle: {} | [T] pose mode: {} | [F] following: {}",
            self.skeleton.joints().len(),
            self.skeleton.bones().len(),
            self.selected
                .map(|cr| if let ColliderRef::Joint(i) = cr {
                    self.skeleton.joints()[i].name().to_string()
                } else {
                    "none".into()
                })
                .unwrap_or_else(|| "none".into()),
            self.edit_mode,
            self.following,
        );
        let clip: &AnimationClip2d = self.clip();
        let playback_line = format!(
            "clips: [{}]\nclip {}: {} {} {:.3}/{:.3} loop: {} | keys: {}\n[Space] play | [K/⇧K] key [←/→] step [G]",
            self.clips
                .iter()
                .map(|c| c.name())
                .collect::<Vec<_>>()
                .join(" | "),
            self.clip().name(),
            if self.playing { "▶" } else { "▮▮" },
            self.clip()
                .easing_at(self.time, SCRUB_TOL)
                .unwrap_or(Easing::Linear),
            self.time,
            clip.duration(),
            if clip.looping() { "on" } else { "off" },
            clip.keyframes().len()
        );
        let vfx = clip.vfx();
        let vfx_line = if vfx.is_empty() {
            "vfx: none".to_string()
        } else {
            let active = vfx
                .iter()
                .filter(|v| {
                    self.time >= v.start_time() && self.time <= v.start_time() + v.duration()
                })
                .map(|v| v.name())
                .collect::<Vec<_>>();
            format!(
                "vfx: {} | [⇧V] {} | active: {}",
                vfx.len(),
                if self.show_vfx { "on" } else { "off" },
                if active.is_empty() {
                    "-".to_string()
                } else {
                    active.join(", ")
                }
            )
        };
        let history_line = format!(
            "undo: {} redo: {}",
            self.history.undo.len(),
            self.history.redo.len(),
        );
        format!(
            "{}\n{}\n{}\n{}\n{}",
            body_line, playback_line, vfx_line, history_line, self.status,
        )
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32().min(0.1);
        self.last_update = now;

        {
            let mut dx: f32 = 0.;
            let mut dy: f32 = 0.;
            if self.keys.cam_up {
                dy += 1.;
            }
            if self.keys.cam_down {
                dy -= 1.;
            }
            if self.keys.cam_right {
                dx += 1.;
            }
            if self.keys.cam_left {
                dx -= 1.;
            }

            if (dx != 0. || dy != 0.) && !self.keys.ctrl_held {
                self.following = false;
                let len = (dx * dx + dy * dy).sqrt();
                dx /= len;
                dy /= len;
                let mut speed = crate::app::CAM_KEY_SPEED * self.camera.view_size;
                if self.keys.shift_held {
                    speed *= 4.;
                }
                self.camera.pos[0] += dx * speed * dt;
                self.camera.pos[1] += dy * speed * dt;
            }
        }

        if self.following {
            let world = self.skeleton.world_transforms(&self.pose);
            if !world.is_empty() {
                let root = world[0].transform_point(al_math::vec::Vec2::ZERO);
                self.camera.update([root.x, root.y], dt);
            }
        }
        if self.playing {
            self.time += dt;

            let (looping, dur) = {
                let clip = self.clip();
                (clip.looping(), clip.duration())
            };
            if looping && dur > 0. {
                self.time = self.time.rem_euclid(dur);
            } else if self.time >= dur {
                self.time = dur;
                self.playing = false;
            }

            if !self.clip().keyframes().is_empty() {
                self.pose = self.clip().sample(self.time);
            }
        }
    }

    pub fn window_event(&mut self, event: winit::event::WindowEvent) {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_px = [position.x as f32, position.y as f32];

                if self.panning {
                    let dx = self.cursor_px[0] - self.pan_last[0];
                    let dy = self.cursor_px[1] - self.pan_last[1];
                    let h = self.resolution[1].max(1.0);
                    let vs = self.camera.view_size;
                    self.camera.pos[0] -= (dx / h) * vs;
                    self.camera.pos[1] += (dy / h) * vs;
                    self.pan_last = self.cursor_px;
                }

                // Recompute world before applying drag
                let cx = self.cursor_px[0] - 0.5 * self.resolution[0];
                let cy = self.cursor_px[1] - 0.5 * self.resolution[1];
                self.cursor_world = Vec2::new(
                    self.camera.pos[0] + (cx / self.resolution[1]) * self.camera.view_size,
                    self.camera.pos[1] + (-cy / self.resolution[1]) * self.camera.view_size,
                );

                if let (Some(ColliderRef::Joint(i)), Some(mode)) = (self.selected, self.drag) {
                    apply_drag(self, i, mode);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                if button == winit::event::MouseButton::Left {
                    if pressed {
                        self.selected = self.pick();
                        self.drag = self.selected.map(|_| DragMode::MoveJoint);
                        self.drag_dirty = false;
                    } else {
                        self.drag = None;
                        self.drag_dirty = false;
                    }
                } else if button == winit::event::MouseButton::Right {
                    if pressed {
                        self.panning = true;
                        self.pan_last = self.cursor_px;
                        self.following = false;
                    } else {
                        self.panning = false;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll = adjust radius of selected joint/bone or zoom if none selected.
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 40.0,
                };
                if let Some(ColliderRef::Joint(i)) = self.selected {
                    if self.scroll_dirty && self.last_scroll.elapsed().as_millis() >= 250 {
                        self.record(Snapshot::Skeleton(self.skeleton.clone()));
                    }
                    if !self.scroll_dirty {
                        self.scroll_dirty = true;
                        self.record(Snapshot::Skeleton(self.skeleton.clone()));
                    }
                    self.last_scroll = Instant::now();
                    let step = 0.002 * scroll;
                    let joints = self.skeleton.joints_mut();
                    let r = joints[i].radius() + step;
                    joints[i].set_radius(r);
                } else {
                    self.camera.view_size =
                        (self.camera.view_size * (1. - 0.1 * scroll)).clamp(0.2, 20.);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.keys.ctrl_held = state.control_key();
                self.keys.shift_held = state.shift_key();
                self.keys.alt_held = state.alt_key();
            }

            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Pressed =>
            {
                use winit::keyboard::KeyCode;
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::KeyW => self.keys.cam_up = true,
                        KeyCode::KeyS => self.keys.cam_down = true,
                        KeyCode::KeyA => self.keys.cam_left = true,
                        KeyCode::KeyD => self.keys.cam_right = true,
                        _ => {}
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Released =>
            {
                use winit::keyboard::KeyCode;
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::KeyW => self.keys.cam_up = false,
                        KeyCode::KeyS => self.keys.cam_down = false,
                        KeyCode::KeyA => self.keys.cam_left = false,
                        KeyCode::KeyD => self.keys.cam_right = false,
                        _ => {}
                    }
                    match code {
                        KeyCode::Tab => {
                            let n = self.skeleton.joints().len();
                            if n > 0 {
                                self.selected = Some(match self.selected {
                                    Some(ColliderRef::Joint(i)) => ColliderRef::Joint((i + 1) % n),
                                    _ => ColliderRef::Joint(0),
                                });
                            }
                        }

                        KeyCode::Space => {
                            self.playing = !self.playing;
                            self.status = if self.playing {
                                "Playing".into()
                            } else {
                                "Paused".into()
                            };
                        }

                        KeyCode::KeyK => {
                            if self.keys.shift_held {
                                self.record(Snapshot::Animation(
                                    self.clips.clone(),
                                    self.current_clip,
                                ));
                                let tol = 1. / 30.;
                                let time = self.time;
                                if self.clip_mut().remove_at(time, tol) {
                                    self.status = format!(
                                        "Key removed ({} left)",
                                        self.clip_mut().keyframes().len()
                                    );
                                } else {
                                    self.status = "No keyframe near playhead.".into();
                                }
                            } else {
                                self.record(Snapshot::Animation(
                                    self.clips.clone(),
                                    self.current_clip,
                                ));
                                let time = self.time;
                                let pose = self.pose.clone();
                                let easing = self.easing;
                                self.clip_mut().insert(time, pose, easing);
                                self.status = format!(
                                    "Key inserted at t={:.3} ({} total)",
                                    self.time,
                                    self.clip().keyframes().len()
                                );
                            }
                        }

                        KeyCode::BracketLeft => self.cycle_clip(-1),
                        KeyCode::BracketRight => self.cycle_clip(1),

                        KeyCode::KeyN if self.keys.ctrl_held => {
                            self.history
                                .record(Snapshot::Animation(self.clips.clone(), self.current_clip));
                            self.clips
                                .push(al_anim::anim_clip::AnimationClip2d::new(format!(
                                    "clip_{}",
                                    self.clips.len()
                                )));
                            self.current_clip = self.clips.len() - 1;
                            self.time = 0.;
                            self.playing = false;
                        }

                        KeyCode::KeyD if self.keys.ctrl_held => {
                            self.history
                                .record(Snapshot::Animation(self.clips.clone(), self.current_clip));
                            let mut c = self.clip().clone();
                            c.set_name(format!("{}.copy", c.name()));
                            self.clips.push(c);
                            self.current_clip = self.clips.len() - 1;
                        }

                        KeyCode::Delete if self.keys.ctrl_held => {
                            if self.clips.is_empty() {
                                return;
                            }
                            self.history
                                .record(Snapshot::Animation(self.clips.clone(), self.current_clip));
                            self.clips.remove(self.current_clip);
                            self.current_clip = self.current_clip.min(self.clips.len() - 1);
                            self.time = 0.;
                        }

                        KeyCode::ArrowLeft => {
                            let step = if self.keys.alt_held {
                                FIXED_DT
                            } else {
                                5. * FIXED_DT
                            };
                            //REIVEW: Cycle to clip duration instead of clamping?
                            self.time = (self.time - step).max(0.);
                            if !self.clip().keyframes().is_empty() {
                                self.pose = self.clip().sample(self.time);
                            }
                        }

                        KeyCode::ArrowRight => {
                            let step = if self.keys.alt_held {
                                FIXED_DT
                            } else {
                                5. * FIXED_DT
                            };
                            self.time = self.time + step;
                            if !self.clip().keyframes().is_empty() {
                                self.pose = self.clip().sample(self.time);
                            }
                        }

                        KeyCode::KeyG => {
                            if let Some(t) = self.clip().nearest_keyframe_time(self.time) {
                                self.time = t;
                                self.pose = self.clip().sample(t);
                            }
                        }

                        KeyCode::Home => {
                            self.time = 0.;
                            if !self.clip().keyframes().is_empty() {
                                self.pose = self.clip().sample(0.);
                            }
                        }

                        KeyCode::KeyL => {
                            let looping = !self.clip().looping();
                            self.clip_mut().set_looping(looping);
                            self.status = format!("Loop: {}", if looping { "on" } else { "off" });
                        }

                        KeyCode::KeyF => self.following = !self.following,

                        KeyCode::KeyT => {
                            self.edit_mode.cycle();
                            self.status = self.edit_mode.status();
                        }

                        // Cycle drag mode
                        KeyCode::KeyM => self.drag = Some(DragMode::MoveJoint),
                        KeyCode::KeyR => self.drag = Some(DragMode::RotateJoint),

                        // Reset root transforms (nudge squash/stretch by hand via keys)
                        KeyCode::KeyQ => {
                            self.record(Snapshot::Pose(self.pose.clone()));
                            self.pose.root_scale = Vec2::new(
                                (self.pose.root_scale.x - 0.05).max(0.2),
                                (self.pose.root_scale.y + 0.05).min(3.0),
                            );
                        }
                        KeyCode::KeyE => {
                            if self.keys.ctrl_held {
                                let current = self.clip().easing_at(self.time, SCRUB_TOL);
                                if let Some(curr) = current {
                                    let next = curr.next();
                                    let time = self.time;
                                    self.clip_mut().set_easing_at(time, next, SCRUB_TOL);
                                    self.status = format!("Easing {curr} -> {next}");
                                } else {
                                    self.status = "No keyframes near playhead.".into();
                                }
                            } else {
                                self.record(Snapshot::Pose(self.pose.clone()));
                                self.pose.root_scale = Vec2::new(
                                    (self.pose.root_scale.x + 0.05).min(3.0),
                                    (self.pose.root_scale.y - 0.05).max(0.2),
                                );
                            }
                        }
                        KeyCode::KeyZ => {
                            if self.keys.ctrl_held {
                                if self.keys.shift_held {
                                    if !self.redo() {
                                        self.status = "Nothing to redo.".into();
                                    }
                                } else {
                                    if !self.undo() {
                                        self.status = "Nothing to undo.".into();
                                    }
                                }
                            } else {
                                self.record(Snapshot::Pose(self.pose.clone()));
                                self.pose.root_scale = Vec2::ONE;
                                self.pose.root_rotation = 0.0;
                                self.pose.root_translation = Vec2::ZERO;
                                self.status = "Root transform reset.".into();
                            }
                        }

                        KeyCode::KeyV if self.keys.shift_held => {
                            self.show_vfx = !self.show_vfx;
                            self.status = format!(
                                "VFX preview: {}",
                                if self.show_vfx { "on" } else { "off" }
                            );
                        }

                        KeyCode::KeyX if self.keys.shift_held => {
                            self.show_extras = !self.show_extras;
                            self.status = format!(
                                "Extras preview: {}",
                                if self.show_extras { "on" } else { "off" }
                            );
                        }

                        KeyCode::KeyA if self.keys.ctrl_held => {
                            if let Some(ColliderRef::Extra(i)) = self.selected {
                                self.extra_active[i] = !self.extra_active[i];
                            }
                        }

                        KeyCode::KeyO if self.keys.ctrl_held => {
                            if self.keys.shift_held {
                                self.onion_keys = (self.onion_keys + 1) % 4;
                                self.status = if self.onion_keys == 0 {
                                    "Onion view: Off".into()
                                } else {
                                    format!(
                                        "Onion view: {} prev/next {}",
                                        self.onion_keys,
                                        match self.onion_mode {
                                            OnionMode::Keys => "keys",
                                            OnionMode::Time => "time",
                                        }
                                    )
                                }
                            } else {
                                self.onion_mode = self.onion_mode.cycle();
                            }
                        }

                        KeyCode::F5 => {
                            if let Err(e) = save_skeleton(self) {
                                self.status = format!("Skeleton save failed: {e}");
                            }
                        }
                        KeyCode::F9 => {
                            if let Err(e) = load_skeleton(self) {
                                self.status = format!("Skeleton load failed: {e}");
                            }
                        }

                        KeyCode::F6 => {
                            if let Err(e) = save_clips(self) {
                                self.status = format!("Animations save failed: {e}");
                            }
                        }
                        KeyCode::F10 => {
                            if let Err(e) = load_clips(self) {
                                self.status = format!("Animations load failed: {e}");
                            }
                        }

                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}

fn apply_drag(designer: &mut DesignerState, i: usize, mode: DragMode) {
    let world = designer.skeleton.world_transforms(&designer.pose);

    // Cursor in the parent's local frame (or world if no parent).
    let target_local = match designer.skeleton.joints()[i].parent() {
        None => designer.cursor_world,
        Some(p) => match world[p].try_inverse() {
            Some(inv) => inv.transform_point(designer.cursor_world),
            None => return,
        },
    };

    match mode {
        DragMode::MoveJoint => {
            match designer.edit_mode {
                crate::designer_state::EditMode::Pose => {
                    // No per-joint translation in Pose; ignore (or add one later).
                    designer.status =
                        "Move in pose mode is a no-op (add per-joint offset to Pose).".into()
                }
                crate::designer_state::EditMode::Rest => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                        designer.drag_dirty = true;
                    }
                    let j = &mut designer.skeleton.joints_mut()[i];
                    j.set_center(target_local);
                }
            }
        }
        DragMode::RotateJoint => {
            // Rotation applied in the parent's local frame.
            let v = target_local - designer.skeleton.joints()[i].center();
            if v.length() < 1e-5 {
                return;
            }
            let angle = v.y.atan2(v.x);

            match designer.edit_mode {
                crate::designer_state::EditMode::Pose => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Pose(designer.pose.clone()));
                        designer.drag_dirty = true;
                    }
                    // Pose rotation is added on top of rest_rotation.
                    designer.pose.rotations[i] = angle - designer.skeleton.joints()[i].rotation();
                }
                crate::designer_state::EditMode::Rest => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                        designer.drag_dirty = true;
                    }
                    designer.skeleton.joints_mut()[i].set_rotation(angle);
                }
            }
        }
    }
}

fn save_skeleton(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = serde_json::to_string_pretty(&designer.skeleton).map_err(|e| e.to_string())?;
        std::fs::write("skeleton.sk", s).map_err(|e| e.to_string())?;
        designer.status = "Skeleton Saved.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn load_skeleton(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = std::fs::read_to_string("skeleton.sk").map_err(|e| e.to_string())?;
        let sk: al_skeleton::skeleton::Skeleton2d =
            serde_json::from_str(&s).map_err(|e| e.to_string())?;
        designer.pose = al_skeleton::pose::Pose2d::rest(sk.joints().len());
        designer.skeleton = sk;
        designer.selected = None;
        designer.status = "Skeleton Loaded.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn save_clips(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = serde_json::to_string_pretty(&designer.clips).map_err(|e| e.to_string())?;
        std::fs::write("clips.am", s).map_err(|e| e.to_string())?;
        designer.status = "Clips Saved.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn load_clips(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = std::fs::read_to_string("clips.am").map_err(|e| e.to_string())?;
        let clips: Vec<al_anim::anim_clip::AnimationClip2d> =
            serde_json::from_str(&s).map_err(|e| e.to_string())?;
        designer.clips = clips;
        designer.status = "Clips Loaded.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}
