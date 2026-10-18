use crate::vec::{Vec2, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable,))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dPrim {
    a: Vec2,
    b: Vec2,
    radius: f32,
    kind: u32,
    _pad: Vec2,
    color: Vec4,
}

impl Sdf2dPrim {
    const CIRCLE: u32 = 0;
    const CAPSULE: u32 = 1;
    const RECT: u32 = 2;

    fn new(a: Vec2, b: Vec2, radius: f32, kind: u32, color: Vec4) -> Self {
        Self {
            a,
            b,
            radius,
            kind,
            _pad: Vec2::ZERO,
            color,
        }
    }

    pub fn circle(center: Vec2, radius: f32, color: Vec4) -> Self {
        Self::new(center, center, radius, Self::CIRCLE, color)
    }

    pub fn capsule(origin: Vec2, tip: Vec2, radius: f32, color: Vec4) -> Self {
        Self::new(origin, tip, radius, Self::CAPSULE, color)
    }

    pub fn rect(center: Vec2, half_size: Vec2, corner_radius: f32, color: Vec4) -> Self {
        Self::new(center, half_size, corner_radius, Self::RECT, color)
    }
}
