///! Gathered from `https://iquilezles.org/articles/distfunctions2d/`
use crate::{
    sdf::Sdf2dPrim,
    transform::Transform2d,
    vec::{Vec2, Vec4},
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dOp {
    round: f32,
    onion: f32,
    elongate: Vec2,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2dBoolean {
    Union,
    Subtraction,
    SmoothUnion(f32),
    SmoothSubtraction(f32),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2d {
    Circle {
        origin: Vec2,
        radius: f32,
    },
    Ellipse {
        origin: Vec2,
        ab: Vec2,
    },
    Capsule {
        origin: Vec2,
        length: f32,
        radius: f32,
    },
    UnevenCapsule {
        origin: Vec2,
        length: f32,
        origin_radius: f32,
        tip_radius: f32,
    },
    Rect {
        origin: Vec2,
        half_size: Vec2,
    },
    RoundedRect {
        origin: Vec2,
        half_size: Vec2,
        corner_radii: Vec4,
    },
    ChamferRect {
        origin: Vec2,
        half_size: Vec2,
        chamfers: Vec4,
    },
    Rhombus {
        origin: Vec2,
        half_size: Vec2,
    },
    Trapezoid {
        origin: Vec2,
        bottom_radius: f32,
        top_radius: f32,
        half_height: f32,
    },
    Parallelogram {
        origin: Vec2,
        width: f32,
        height: f32,
        skew: f32,
    },
    Triangle {
        origin: Vec2,
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
    },
    EquTriangle {
        origin: Vec2,
        radius: f32,
    },
    IsoTriangle {
        origin: Vec2,
        height: f32,
        half_base: f32,
    },
    Pentagon {
        origin: Vec2,
        radius: f32,
    },
    Hexagon {
        origin: Vec2,
        radius: f32,
    },
    Octogon {
        origin: Vec2,
        radius: f32,
    },
    Pentragram {
        origin: Vec2,
        radius: f32,
    },
    Hexagram {
        origin: Vec2,
        radius: f32,
    },
    Star {
        origin: Vec2,
        outer_radius: f32,
        /// Must be between 2 and `point_count`
        inner_radius_factor: f32,
        point_count: u32,
    },
    Pie {
        origin: Vec2,
        open_angle: Vec2,
        radius: f32,
    },
    CutDisk {
        origin: Vec2,
        radius: f32,
        cut_height: f32,
    },
    Arc {
        origin: Vec2,
        open_angle: Vec2,
        radius: f32,
        thickness: f32,
    },
    Ring {
        origin: Vec2,
        normal: Vec2,
        radius: f32,
        thickness: f32,
    },
    Horseshoe {
        origin: Vec2,
        open_angle: Vec2,
        radius: f32,
        thickness: Vec2,
    },
    Vesica {
        origin: Vec2,
        width: f32,
        height: f32,
    },
    Moon {
        origin: Vec2,
        center_distance: f32,
        outer_radius: f32,
        inner_radius: f32,
    },
    Egg {
        origin: Vec2,
        half_height: f32,
        top_radius: f32,
        bottom_radius: f32,
        bulge: f32,
    },
    Heart {
        origin: Vec2,
    },
    RoundedX {
        origin: Vec2,
        thickness: f32,
        corner_radius: f32,
    },
    Cross {
        origin: Vec2,
        arm_half_extent: Vec2,
        corner_radius: f32,
    },
    CircleCross {
        origin: Vec2,
        arm_extent: f32,
    },
    BlobbyCross {
        origin: Vec2,
        half_extent: f32,
    },
    Parabola {
        origin: Vec2,
        curvature: f32,
    },
    ParabolaSegment {
        origin: Vec2,
        half_width: f32,
        height: f32,
    },
    Hyperbola {
        origin: Vec2,
        curvature: f32,
        half_length: f32,
    },
    QuadraticBezier {
        origin: Vec2,
        point_a: Vec2,
        point_b: Vec2,
        point_c: Vec2,
    },
    QuadraticCircle {
        origin: Vec2,
    },
    Tunnel {
        origin: Vec2,
        half_extents: Vec2,
    },
    Stairs {
        origin: Vec2,
        step_size: Vec2,
        count: f32,
    },
    CircleWave {
        origin: Vec2,
        wave_angle: f32,
        radius: f32,
    },
}

impl Sdf2d {
    pub fn circle(origin: Vec2, radius: f32) -> Self {
        Self::Circle { origin, radius }
    }

    pub fn capsule(origin: Vec2, length: f32, radius: f32) -> Self {
        Self::Capsule {
            origin,
            length,
            radius,
        }
    }

    pub fn rect(origin: Vec2, half_size: Vec2) -> Self {
        Self::Rect { origin, half_size }
    }

    pub fn rounded_rect(origin: Vec2, half_size: Vec2, corner_radii: Vec4) -> Self {
        Self::RoundedRect {
            origin,
            half_size,
            corner_radii,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Sdf2d::Circle { .. } => "Circle",
            Sdf2d::Ellipse { .. } => "Ellipse",
            Sdf2d::Capsule { .. } => "Capsule",
            Sdf2d::UnevenCapsule { .. } => "UnevenCapsule",
            Sdf2d::Rect { .. } => "Rect",
            Sdf2d::RoundedRect { .. } => "RoundedRect",
            Sdf2d::ChamferRect { .. } => "ChamferRect",
            Sdf2d::Rhombus { .. } => "Rhombus",
            Sdf2d::Trapezoid { .. } => "Trapezoid",
            Sdf2d::Parallelogram { .. } => "Parallelogram",
            Sdf2d::Triangle { .. } => "Triangle",
            Sdf2d::EquTriangle { .. } => "EquTriangle",
            Sdf2d::IsoTriangle { .. } => "IsoTriangle",
            Sdf2d::Pentagon { .. } => "Pentagon",
            Sdf2d::Hexagon { .. } => "Hexagon",
            Sdf2d::Octogon { .. } => "Octogon",
            Sdf2d::Pentragram { .. } => "Pentragram",
            Sdf2d::Hexagram { .. } => "Hexagram",
            Sdf2d::Star { .. } => "Star",
            Sdf2d::Pie { .. } => "Pie",
            Sdf2d::CutDisk { .. } => "CutDisk",
            Sdf2d::Arc { .. } => "Arc",
            Sdf2d::Ring { .. } => "Ring",
            Sdf2d::Horseshoe { .. } => "Horseshoe",
            Sdf2d::Vesica { .. } => "Vesica",
            Sdf2d::Moon { .. } => "Moon",
            Sdf2d::Egg { .. } => "Egg",
            Sdf2d::Heart { .. } => "Heart",
            Sdf2d::RoundedX { .. } => "RoundedX",
            Sdf2d::Cross { .. } => "Cross",
            Sdf2d::CircleCross { .. } => "CircleCross",
            Sdf2d::BlobbyCross { .. } => "BobblyCross",
            Sdf2d::Parabola { .. } => "Parabola",
            Sdf2d::ParabolaSegment { .. } => "ParabolaSegment",
            Sdf2d::Hyperbola { .. } => "Hyperbola",
            Sdf2d::QuadraticBezier { .. } => "QuadraticBezier",
            Sdf2d::QuadraticCircle { .. } => "QuadraticCircle",
            Sdf2d::Tunnel { .. } => "Tunnel",
            Sdf2d::Stairs { .. } => "Stairs",
            Sdf2d::CircleWave { .. } => "CircleWave",
        }
    }

    pub fn to_prim(self, world: Transform2d, color: Vec4) -> Sdf2dPrim {
        let m_inv = world
            .linear
            .try_inverse()
            .unwrap_or(crate::matrix::Mat2::IDENTITY)
            .to_cols_array();
        match self {
            Self::Circle {
                origin: center,
                radius,
            } => Sdf2dPrim::circle(world.transform_point(center), radius, m_inv, color),
            Self::Ellipse { origin, ab } => todo!(),
            Self::Capsule {
                origin,
                length,
                radius,
            } => Sdf2dPrim::capsule(world.transform_point(origin), length, radius, m_inv, color),
            Sdf2d::UnevenCapsule {
                origin,
                length,
                origin_radius,
                tip_radius,
            } => todo!(),
            Self::Rect {
                origin: center,
                half_size,
            } => Sdf2dPrim::rect(world.transform_point(center), half_size, m_inv, color),
            Self::RoundedRect {
                origin: center,
                half_size,
                corner_radii,
            } => todo!(),
            Self::ChamferRect {
                origin,
                half_size,
                chamfers,
            } => todo!(),
            Sdf2d::Rhombus { origin, half_size } => todo!(),
            Sdf2d::Trapezoid {
                origin,
                bottom_radius,
                top_radius,
                half_height,
            } => todo!(),
            Sdf2d::Parallelogram {
                origin,
                width,
                height,
                skew,
            } => todo!(),
            Sdf2d::Triangle { origin, p0, p1, p2 } => todo!(),
            Sdf2d::EquTriangle { origin, radius } => todo!(),
            Sdf2d::IsoTriangle {
                origin,
                height,
                half_base,
            } => todo!(),
            Sdf2d::Pentagon { origin, radius } => todo!(),
            Sdf2d::Hexagon { origin, radius } => todo!(),
            Sdf2d::Octogon { origin, radius } => todo!(),
            Sdf2d::Pentragram { origin, radius } => todo!(),
            Sdf2d::Hexagram { origin, radius } => todo!(),
            Sdf2d::Star {
                origin,
                outer_radius,
                inner_radius_factor,
                point_count,
            } => todo!(),
            Sdf2d::Pie {
                origin,
                open_angle,
                radius,
            } => todo!(),
            Sdf2d::CutDisk {
                origin,
                radius,
                cut_height,
            } => todo!(),
            Sdf2d::Arc {
                origin,
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Ring {
                origin,
                normal,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Horseshoe {
                origin,
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Vesica {
                origin,
                width,
                height,
            } => todo!(),
            Sdf2d::Moon {
                origin,
                center_distance,
                outer_radius,
                inner_radius,
            } => todo!(),
            Sdf2d::Egg {
                origin,
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => todo!(),
            Sdf2d::Heart { origin } => todo!(),
            Sdf2d::RoundedX {
                origin,
                thickness,
                corner_radius,
            } => todo!(),
            Sdf2d::Cross {
                origin,
                arm_half_extent,
                corner_radius,
            } => todo!(),
            Sdf2d::CircleCross { origin, arm_extent } => todo!(),
            Sdf2d::BlobbyCross {
                origin,
                half_extent,
            } => todo!(),
            Sdf2d::Parabola { origin, curvature } => todo!(),
            Sdf2d::ParabolaSegment {
                origin,
                half_width,
                height,
            } => todo!(),
            Sdf2d::Hyperbola {
                origin,
                curvature,
                half_length,
            } => todo!(),
            Sdf2d::QuadraticBezier {
                origin,
                point_a,
                point_b,
                point_c,
            } => todo!(),
            Sdf2d::QuadraticCircle { origin } => todo!(),
            Sdf2d::Tunnel {
                origin,
                half_extents,
            } => todo!(),
            Sdf2d::Stairs {
                origin,
                step_size,
                count,
            } => todo!(),
            Sdf2d::CircleWave {
                origin,
                wave_angle,
                radius,
            } => todo!(),
        }
    }

    pub fn accumulate(
        point: Vec2,
        prims: &[Self],
        ops: &[Sdf2dOp],
        booleans: &[Sdf2dBoolean],
    ) -> f32 {
        let mut d = f32::INFINITY;
        for (i, prim) in prims.iter().enumerate() {
            let d_prim = prim.eval(point, ops[i]);
            d = match booleans[i] {
                Sdf2dBoolean::Union | Sdf2dBoolean::SmoothUnion(_) => d.min(d_prim),
                Sdf2dBoolean::Subtraction | Sdf2dBoolean::SmoothSubtraction(_) => d.max(-d_prim),
            };
        }
        d
    }

    pub fn eval(self, point: Vec2, ops: Sdf2dOp) -> f32 {
        // elongate
        let q = point - point.clamp(-ops.elongate, ops.elongate);
        let mut d = self.dist(q);

        // onion
        if ops.onion > 0. {
            d = d.abs() - ops.onion;
        }

        // round
        d - ops.round
    }

    pub fn dist(self, point: Vec2) -> f32 {
        match self {
            Sdf2d::Circle { origin, radius } => circle_sdf_2d(point, radius, origin),
            Sdf2d::Ellipse { origin, ab } => todo!(),
            Sdf2d::Capsule {
                origin,
                length,
                radius,
            } => capsule_sdf_2d(point, radius, origin, length),
            Sdf2d::UnevenCapsule {
                origin,
                length,
                origin_radius,
                tip_radius,
            } => todo!(),
            Sdf2d::Rect { origin, half_size } => rect_sdf_2d(point, origin, half_size),
            Sdf2d::RoundedRect {
                origin,
                half_size,
                corner_radii,
            } => rounded_rect_sdf_2d(point, origin, half_size, corner_radii),
            Sdf2d::ChamferRect {
                origin,
                half_size,
                chamfers,
            } => todo!(),
            Sdf2d::Rhombus { origin, half_size } => todo!(),
            Sdf2d::Trapezoid {
                origin,
                bottom_radius,
                top_radius,
                half_height,
            } => todo!(),
            Sdf2d::Parallelogram {
                origin,
                width,
                height,
                skew,
            } => todo!(),
            Sdf2d::Triangle { origin, p0, p1, p2 } => todo!(),
            Sdf2d::EquTriangle { origin, radius } => todo!(),
            Sdf2d::IsoTriangle {
                origin,
                height,
                half_base,
            } => todo!(),
            Sdf2d::Pentagon { origin, radius } => todo!(),
            Sdf2d::Hexagon { origin, radius } => todo!(),
            Sdf2d::Octogon { origin, radius } => todo!(),
            Sdf2d::Pentragram { origin, radius } => todo!(),
            Sdf2d::Hexagram { origin, radius } => todo!(),
            Sdf2d::Star {
                origin,
                outer_radius,
                inner_radius_factor,
                point_count,
            } => todo!(),
            Sdf2d::Pie {
                origin,
                open_angle,
                radius,
            } => todo!(),
            Sdf2d::CutDisk {
                origin,
                radius,
                cut_height,
            } => todo!(),
            Sdf2d::Arc {
                origin,
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Ring {
                origin,
                normal,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Horseshoe {
                origin,
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2d::Vesica {
                origin,
                width,
                height,
            } => todo!(),
            Sdf2d::Moon {
                origin,
                center_distance,
                outer_radius,
                inner_radius,
            } => todo!(),
            Sdf2d::Egg {
                origin,
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => todo!(),
            Sdf2d::Heart { origin } => todo!(),
            Sdf2d::RoundedX {
                origin,
                thickness,
                corner_radius,
            } => todo!(),
            Sdf2d::Cross {
                origin,
                arm_half_extent,
                corner_radius,
            } => todo!(),
            Sdf2d::CircleCross { origin, arm_extent } => todo!(),
            Sdf2d::BlobbyCross {
                origin,
                half_extent,
            } => todo!(),
            Sdf2d::Parabola { origin, curvature } => todo!(),
            Sdf2d::ParabolaSegment {
                origin,
                half_width,
                height,
            } => todo!(),
            Sdf2d::Hyperbola {
                origin,
                curvature,
                half_length,
            } => todo!(),
            Sdf2d::QuadraticBezier {
                origin,
                point_a,
                point_b,
                point_c,
            } => todo!(),
            Sdf2d::QuadraticCircle { origin } => todo!(),
            Sdf2d::Tunnel {
                origin,
                half_extents,
            } => todo!(),
            Sdf2d::Stairs {
                origin,
                step_size,
                count,
            } => todo!(),
            Sdf2d::CircleWave {
                origin,
                wave_angle,
                radius,
            } => todo!(),
        }
    }
}

#[inline]
pub fn circle_sdf_2d(point: Vec2, radius: f32, origin: Vec2) -> f32 {
    (point - origin).length() - radius
}

#[inline]
pub fn capsule_sdf_2d(point: Vec2, radius: f32, origin: Vec2, length: f32) -> f32 {
    let h = point.x.clamp(0., length);
    (point - Vec2::new(h, 0.)).length() - radius
}

#[inline]
pub fn capsule_tip_sdf_2d(point: Vec2, radius: f32, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let h = ((point - a).dot(ab) / ab.dot(ab).max(1e-8)).clamp(0., 1.);
    (point - (a + ab * h)).length() - radius
}

#[inline]
pub fn rect_sdf_2d(point: Vec2, origin: Vec2, half_size: Vec2) -> f32 {
    let q = (point - origin).abs() - half_size;
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.)
}

#[inline]
pub fn rounded_rect_sdf_2d(point: Vec2, origin: Vec2, half_size: Vec2, corner_radii: Vec4) -> f32 {
    let mut r = if point.x > 0. {
        Vec2::new(corner_radii.x, corner_radii.y)
    } else {
        Vec2::new(corner_radii.z, corner_radii.w)
    };
    if point.y <= 0. {
        r.x = r.y;
    }
    let q = (point - origin).abs() - half_size + Vec2::from(r.x);
    return q.x.max(q.y).min(0.) + q.max(Vec2::ZERO).length() - r.x;
}

#[inline]
pub fn chamfer_rect_sdf_2d() -> f32 {
    todo!()
}

#[inline]
pub fn rhombus_sdf_2d() -> f32 {
    todo!()
}

#[inline]
pub fn trapezoid_sdf_2d() -> f32 {
    todo!()
}

#[inline]
pub fn parallelogram_sdf_2d() -> f32 {
    todo!()
}

#[inline]
pub fn triangle_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn equ_triangle_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn iso_triangle_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn pentagon_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn hexagon_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn octogon_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn pentragram_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn hexagram_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn star_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn pie_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn cut_disk_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn arc_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn ring_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn horseshoe_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn vesica_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn moon_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn egg_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn heart_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn rounded_x_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn cross_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn circle_cross_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn bobbly_cross_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn parabola_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn parabola_segment_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn hyperbola_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn quadratic_bezier_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn quadratic_circle_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn tunnel_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn stairs_sdf_2d() -> f32 {
    todo!();
}

#[inline]
pub fn circle_wave_sdf_2d() -> f32 {
    todo!();
}
