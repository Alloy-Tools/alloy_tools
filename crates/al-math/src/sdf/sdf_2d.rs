use crate::{
    sdf::Sdf2dPrim,
    transform::Transform2d,
    vec::{Vec2, Vec4},
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2d {
    Circle {
        center: Vec2,
        radius: f32,
    },
    Capsule {
        origin: Vec2,
        tip: Vec2,
        radius: f32,
    },
    Rect {
        center: Vec2,
        half_size: Vec2,
        corner_radius: f32,
    },
}

impl Sdf2d {
    pub fn circle(center: Vec2, radius: f32) -> Self {
        Self::Circle { center, radius }
    }

    pub fn capsule(origin: Vec2, tip: Vec2, radius: f32) -> Self {
        Self::Capsule {
            origin,
            tip,
            radius,
        }
    }

    pub fn rect(center: Vec2, half_size: Vec2, corner_radius: f32) -> Self {
        Self::Rect {
            center,
            half_size,
            corner_radius,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Sdf2d::Circle { .. } => "Circle",
            Sdf2d::Capsule { .. } => "Capsule",
            Sdf2d::Rect { .. } => "Rect",
        }
    }

    pub fn to_prim(self, world: Transform2d, color: Vec4) -> Sdf2dPrim {
        let (_, rot, scale) = world.to_components();
        //REVIEW: Only uniform scale is currently implemented
        let s = scale.x.min(scale.y);
        match self {
            Self::Circle { center, radius } => {
                Sdf2dPrim::circle(world.transform_point(center), radius * s, color)
            }
            Self::Capsule {
                origin,
                tip,
                radius,
            } => {
                let wo = world.transform_point(origin);
                let wt = world.transform_point(tip);
                Sdf2dPrim::capsule(wo, wt - wo, radius * s, color)
            }
            Self::Rect {
                center,
                half_size,
                corner_radius,
            } => Sdf2dPrim::rect(
                world.transform_point(center),
                half_size * s,
                corner_radius * s,
                rot,
                color,
            ),
        }
    }
}

#[inline]
pub fn circle_sdf_2d(point: Vec2, radius: f32, center: Vec2) -> f32 {
    (point - center).length() - radius
}

#[inline]
pub fn capsule_sdf_2d(point: Vec2, radius: f32, origin: Vec2, tip: Vec2) -> f32 {
    let po = point - origin;
    let to = tip - origin;
    let h = (po.dot(to) / to.dot(to).max(1e-8)).clamp(0., 1.);
    (po - to * h).length() - radius
}

#[inline]
pub fn rect_sdf_2d(point: Vec2, center: Vec2, half_size: Vec2, corner_radius: f32) -> f32 {
    let q = (point - center).abs() - half_size + Vec2::new(corner_radius, corner_radius);
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.) - corner_radius
}
