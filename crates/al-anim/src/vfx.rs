use crate::{anim_clip::AttachType, Easing};
use al_math::{
    sdf::Sdf2d,
    vec::{Vec2, Vec3, Vec4},
};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vfx2d {
    name: String,
    start_time: f32,
    duration: f32,
    attach: AttachType,
    shape: Sdf2d,
    // Scale, Alpha, Rotation
    start: Vec3,
    end: Vec3,
    motion: Vec2,
    color: Vec4,
    easing: Easing,
}

impl Vfx2d {
    pub fn new(
        name: impl Into<String>,
        start_time: f32,
        duration: f32,
        attach: AttachType,
        shape: Sdf2d,
        scale: (f32, f32),
        alpha: (f32, f32),
        rotation: (f32, f32),
        motion: Vec2,
        color: Vec4,
        easing: Easing,
    ) -> Self {
        Self {
            name: name.into(),
            start_time,
            duration,
            attach,
            shape,
            start: Vec3::new(scale.0, alpha.0, rotation.0),
            end: Vec3::new(scale.1, alpha.1, rotation.1),
            motion,
            color,
            easing,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn start_time(&self) -> f32 {
        self.start_time
    }

    pub fn set_start_time(&mut self, time: f32) {
        self.start_time = time;
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn set_duration(&mut self, duration: f32) {
        self.duration = duration;
    }

    pub fn attach_type(&self) -> AttachType {
        self.attach
    }

    pub fn set_attach_type(&mut self, attach: AttachType) {
        self.attach = attach;
    }

    pub fn shape(&self) -> Sdf2d {
        self.shape
    }

    pub fn set_shape(&mut self, shape: Sdf2d) {
        self.shape = shape;
    }

    pub fn start_scale(&self) -> f32 {
        self.start.x
    }

    pub fn set_start_scale(&mut self, scale: f32) {
        self.start.x = scale;
    }

    pub fn start_alpha(&self) -> f32 {
        self.start.y
    }

    pub fn set_start_alpha(&mut self, alpha: f32) {
        self.start.y = alpha;
    }

    pub fn start_rotation(&self) -> f32 {
        self.start.z
    }

    pub fn set_start_rotation(&mut self, rotation: f32) {
        self.start.z = rotation;
    }

    pub fn end_scale(&self) -> f32 {
        self.end.x
    }

    pub fn set_end_scale(&mut self, scale: f32) {
        self.end.x = scale;
    }

    pub fn end_alpha(&self) -> f32 {
        self.end.y
    }

    pub fn set_end_alpha(&mut self, alpha: f32) {
        self.end.y = alpha;
    }

    pub fn end_rotation(&self) -> f32 {
        self.end.z
    }

    pub fn set_end_rotation(&mut self, rotation: f32) {
        self.end.z = rotation;
    }

    pub fn motion(&self) -> Vec2 {
        self.motion
    }

    pub fn set_motion(&mut self, motion: Vec2) {
        self.motion = motion;
    }

    pub fn color(&self) -> Vec4 {
        self.color
    }

    pub fn set_color(&mut self, color: Vec4) {
        self.color = color;
    }

    pub fn easing(&self) -> Easing {
        self.easing
    }

    pub fn set_easing(&mut self, easing: Easing) {
        self.easing = easing;
    }
}
