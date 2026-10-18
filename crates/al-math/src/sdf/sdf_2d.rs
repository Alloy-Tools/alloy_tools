use crate::{transform::Transform2d, vec::Vec2};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2d {
    Circle {
        origin: Transform2d,
        radius: f32,
    },
    Capsule {
        origin: Transform2d,
        tip: Transform2d,
        radius: f32,
    },
}

impl Sdf2d {
    pub fn circle(origin: Transform2d, radius: f32) -> Self {
        Self::Circle { origin, radius }
    }

    pub fn capsule(origin: Transform2d, tip: Transform2d, radius: f32) -> Self {
        Self::Capsule {
            origin,
            tip,
            radius,
        }
    }

    pub fn radius(&self) -> f32 {
        match self {
            Sdf2d::Circle { radius, .. } => *radius,
            Sdf2d::Capsule { radius, .. } => *radius,
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        *match self {
            Sdf2d::Circle { radius, .. } => radius,
            Sdf2d::Capsule { radius, .. } => radius,
        } = radius;
    }

    pub fn origin(&self) -> Transform2d {
        match self {
            Sdf2d::Circle { origin, .. } => *origin,
            Sdf2d::Capsule { origin, .. } => *origin,
        }
    }

    pub fn set_origin(&mut self, origin: Transform2d) {
        *match self {
            Sdf2d::Circle { origin, .. } => origin,
            Sdf2d::Capsule { origin, .. } => origin,
        } = origin;
    }

    pub fn origin_mut(&mut self) -> &mut Transform2d {
        match self {
            Sdf2d::Circle { origin, .. } => origin,
            Sdf2d::Capsule { origin, .. } => origin,
        }
    }
}

#[inline]
pub fn circle_sdf_2d(radius: f32, point: Vec2, origin: Vec2) -> f32 {
    (point - origin).length() - radius
}

#[inline]
pub fn capsule_sdf_2d(radius: f32, point: Vec2, origin: Vec2, tip: Vec2) -> f32 {
    let po = point - origin;
    let to = tip - origin;
    let h = (po.dot(to) / to.dot(to).max(1e-8)).clamp(0., 1.);
    (po - to * h).length() - radius
}
