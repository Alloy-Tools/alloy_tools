use crate::camera::Camera;
use al_math::vec::Vec2;
use std::time::Instant;

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

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn last_tick(&self) -> Instant {
        self.last_tick
    }

    pub fn tick(&mut self) {
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
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct InputState {
    pub ctrl_held: bool,
    pub shift_held: bool,
    pub alt_held: bool,
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

    pub fn keys(&self) -> CameraKeys {
        self.keys
    }

    pub fn following(&self) -> bool {
        self.following
    }

    pub fn target(&self) -> Option<Vec2> {
        self.target
    }

    pub fn panning(&self) -> bool {
        self.panning
    }

    pub fn pan_last(&self) -> Vec2 {
        self.pan_last
    }

    pub fn cursor_px(&self) -> Vec2 {
        self.cursor_px
    }

    pub fn cursor_world(&self) -> Vec2 {
        self.cursor_world
    }

    pub fn resolution(&self) -> Vec2 {
        self.resolution
    }
}
