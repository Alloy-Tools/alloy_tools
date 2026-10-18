use crate::{
    sdf::sdf_2d::Sdf2d,
    vec::{Vec2, Vec4},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable,))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dPrim {
    origin: Vec2,
    tip: Vec2,
    radius: f32,
    kind: u32,
    _pad: Vec2,
    color: Vec4,
}

impl Sdf2dPrim {
    const CIRCLE: u32 = 0;
    const CAPSULE: u32 = 1;

    fn new(origin: Vec2, tip: Vec2, radius: f32, kind: u32, color: Vec4) -> Self {
        Self {
            origin,
            tip,
            radius,
            kind,
            _pad: Vec2::ZERO,
            color,
        }
    }

    pub fn from_shape(shape: Sdf2d, transform: impl Fn(Vec2) -> Vec2, color: Vec4) -> Self {
        match shape {
            Sdf2d::Circle { origin, radius } => {
                Self::circle(transform(origin.to_components().0), radius, color)
            }
            Sdf2d::Capsule {
                origin,
                tip,
                radius,
            } => Self::capsule(
                transform(origin.to_components().0),
                transform(tip.to_components().0),
                radius,
                color,
            ),
        }
    }

    pub fn circle(origin: Vec2, radius: f32, color: Vec4) -> Self {
        Self::new(origin, origin, radius, Self::CIRCLE, color)
    }

    pub fn capsule(origin: Vec2, tip: Vec2, radius: f32, color: Vec4) -> Self {
        Self::new(origin, tip, radius, Self::CAPSULE, color)
    }
}
