use crate::camera::Camera;
use al_math::{sdf::Sdf2dPrim, vec::Vec2};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct RenderData {
    pub globals: Globals,
    pub sdfs: Vec<Sdf2dPrim>,
    pub hud_string: String,
}

impl RenderData {
    pub fn new(globals: Globals, sdfs: Vec<Sdf2dPrim>, hud_string: impl Into<String>) -> Self {
        Self {
            globals,
            sdfs,
            hud_string: hud_string.into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub resolution: Vec2,
    pub camera_pos: Vec2,
    pub cursor_world: Vec2,
    pub view_size: f32,
    pub aux: f32,
}

impl Globals {
    pub fn new(
        resolution: impl Into<Vec2>,
        camera_pos: impl Into<Vec2>,
        cursor_world: impl Into<Vec2>,
        view_size: f32,
    ) -> Self {
        Self {
            resolution: resolution.into(),
            camera_pos: camera_pos.into(),
            cursor_world: cursor_world.into(),
            view_size,
            aux: 0.,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SdfHeader {
    pub count: u32,
    pub _pad: [u32; 3],
}

impl SdfHeader {
    pub fn new(count: u32) -> Self {
        Self {
            count,
            _pad: [0; 3],
        }
    }
}

pub const SDF_HEADER_SIZE: usize = std::mem::size_of::<SdfHeader>();

#[derive(Clone, Copy, Debug)]
pub struct Timeline {
    time: f32,
    duration: f32,
    looping: bool,
    playing: bool,
    last_tick: Instant,
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new(0.)
    }
}

impl Timeline {
    pub fn new(duration: f32) -> Self {
        Self {
            time: 0.,
            duration,
            looping: false,
            playing: false,
            last_tick: Instant::now(),
        }
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }

    pub fn scrub(&mut self, dt: f32) {
        self.time = (self.time + dt).max(0.);
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn set_duration(&mut self, duration: f32) {
        self.duration = duration;
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    pub fn last_tick(&self) -> Instant {
        self.last_tick
    }

    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().min(0.1);
        self.last_tick = now;

        if self.playing {
            self.time += dt;
            let dur = self.duration;

            if self.looping && dur > 0. {
                self.time = self.time.rem_euclid(dur);
            } else if self.time >= dur {
                self.time = dur;
                self.playing = false;
            }
        }
        dt
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct InputState {
    pub ctrl_held: bool,
    pub shift_held: bool,
    pub alt_held: bool,
    //TODO: Remove
    pub cam_up: bool,
    pub cam_down: bool,
    pub cam_left: bool,
    pub cam_right: bool,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct CameraKeys {
    pub cam_up: bool,
    pub cam_down: bool,
    pub cam_left: bool,
    pub cam_right: bool,
}

pub struct CameraState {
    camera: Camera,
    keys: CameraKeys,
    following: bool,
    target: Option<Vec2>,
    panning: bool,
    pan_last: Vec2,
    cursor_px: Vec2,
    cursor_world: Vec2,
    resolution: Vec2,
}

impl CameraState {
    pub fn new(resolution: impl Into<Vec2>, camera: Camera) -> Self {
        Self {
            camera,
            keys: CameraKeys::default(),
            following: false,
            target: None,
            panning: false,
            pan_last: Vec2::ZERO,
            cursor_px: Vec2::ZERO,
            cursor_world: Vec2::ZERO,
            resolution: resolution.into(),
        }
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn keys(&self) -> CameraKeys {
        self.keys
    }

    pub fn keys_mut(&mut self) -> &mut CameraKeys {
        &mut self.keys
    }

    pub fn following(&self) -> bool {
        self.following
    }

    pub fn set_following(&mut self, following: bool) {
        self.following = following;
    }

    pub fn target(&self) -> Option<Vec2> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<Vec2>) {
        self.target = target;
    }

    pub fn panning(&self) -> bool {
        self.panning
    }

    pub fn set_panning(&mut self, panning: bool) {
        self.panning = panning;
    }

    pub fn pan_last(&self) -> Vec2 {
        self.pan_last
    }

    pub fn set_pan_last(&mut self, pan_last: impl Into<Vec2>) {
        self.pan_last = pan_last.into();
    }

    pub fn cursor_px(&self) -> Vec2 {
        self.cursor_px
    }

    pub fn set_cursor_px(&mut self, cursor_px: impl Into<Vec2>) {
        self.cursor_px = cursor_px.into();
    }

    pub fn cursor_world(&self) -> Vec2 {
        self.cursor_world
    }

    pub fn set_cursor_world(&mut self, cursor_world: impl Into<Vec2>) {
        self.cursor_world = cursor_world.into();
    }

    pub fn resolution(&self) -> Vec2 {
        self.resolution
    }

    pub fn set_resolution(&mut self, resolution: impl Into<Vec2>) {
        self.resolution = resolution.into();
    }

    pub fn update(&mut self, dt: f32, speed_gate: bool) {
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

        if dx != 0. || dy != 0. {
            self.following = false;
            let len = (dx * dx + dy * dy).sqrt();
            dx /= len;
            dy /= len;
            let mut speed = crate::app::CAM_KEY_SPEED * self.camera.view_size;
            if speed_gate {
                speed *= 4.;
            }
            self.camera.pos[0] += dx * speed * dt;
            self.camera.pos[1] += dy * speed * dt;
        }

        if self.following {
            if let Some(target) = self.target {
                self.camera.update(target, dt);
            }
        }
    }
}
