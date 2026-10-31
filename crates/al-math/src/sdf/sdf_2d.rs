///! Gathered from `https://iquilezles.org/articles/distfunctions2d/`
use crate::{
    sdf::Sdf2dPrim,
    transform::Transform2d,
    vec::{Vec2, Vec4},
};

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dOp {
    round: f32,
    onion: f32,
    elongate: Vec2,
}

impl Sdf2dOp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn round(&self) -> f32 {
        self.round
    }

    pub fn with_round(&mut self, round: f32) {
        self.round = round
    }

    pub fn onion(&self) -> f32 {
        self.onion
    }

    pub fn with_onion(&mut self, onion: f32) {
        self.onion = onion
    }

    pub fn elongate(&self) -> Vec2 {
        self.elongate
    }

    pub fn with_elongate(&mut self, elongate: Vec2) {
        self.elongate = elongate
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2dBoolean {
    #[default]
    Union,
    Subtraction,
    SmoothUnion(f32),
    SmoothSubtraction(f32),
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2d {
    transform: Transform2d,
    shape: Sdf2dKind,
}

impl Sdf2d {
    pub fn new(transform: Transform2d, shape: Sdf2dKind) -> Self {
        Self { transform, shape }
    }

    pub fn identity(shape: Sdf2dKind) -> Self {
        Self::new(Transform2d::IDENTITY, shape)
    }

    pub fn transform(&self) -> Transform2d {
        self.transform
    }

    pub fn transform_mut(&mut self) -> &mut Transform2d {
        &mut self.transform
    }

    pub fn shape(&self) -> Sdf2dKind {
        self.shape
    }

    pub fn shape_mut(&mut self) -> &mut Sdf2dKind {
        &mut self.shape
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
        self.eval_composed(self.transform, point, ops)
    }

    pub fn eval_in_parent(self, parent: Option<&Transform2d>, point: Vec2, ops: Sdf2dOp) -> f32 {
        self.eval_composed(
            match parent {
                Some(p) => p.compose(&self.transform),
                None => self.transform,
            },
            point,
            ops,
        )
    }

    fn eval_composed(self, transform: Transform2d, point: Vec2, ops: Sdf2dOp) -> f32 {
        let (local, scale) = match transform.try_inverse() {
            Some(inv) => (inv.transform_point(point), inv.linear.cols[0].length()),
            None => (point, 1.),
        };
        // elongate
        let q = local - local.clamp(-ops.elongate, ops.elongate);
        let mut d = self.shape.dist(q);

        if ops.onion > 0. {
            d = d.abs() - ops.onion;
        }
        d = d - ops.round;

        d * scale
    }

    pub fn to_prim(self, world: Transform2d, color: Vec4) -> Sdf2dPrim {
        let composed = world.compose(&self.transform);
        let m_inv = composed
            .linear
            .try_inverse()
            .unwrap_or(crate::matrix::Mat2::IDENTITY)
            .to_cols_array();
        match self.shape {
            Sdf2dKind::Circle { radius } => {
                Sdf2dPrim::circle(composed.translation, radius, m_inv, color)
            }
            Sdf2dKind::Ellipse { ab } => todo!(),
            Sdf2dKind::Capsule { length, radius } => {
                Sdf2dPrim::capsule(composed.translation, length, radius, m_inv, color)
            }
            Sdf2dKind::UnevenCapsule {
                length,
                origin_radius,
                tip_radius,
            } => todo!(),
            Sdf2dKind::Rect { half_size } => {
                Sdf2dPrim::rect(composed.translation, half_size, m_inv, color)
            }
            Sdf2dKind::RoundedRect {
                half_size,
                corner_radii,
            } => todo!(),
            Sdf2dKind::ChamferRect {
                half_size,
                chamfers,
            } => todo!(),
            Sdf2dKind::Rhombus { half_size } => todo!(),
            Sdf2dKind::Trapezoid {
                bottom_radius,
                top_radius,
                half_height,
            } => todo!(),
            Sdf2dKind::Parallelogram {
                width,
                height,
                skew,
            } => todo!(),
            Sdf2dKind::Triangle { p0, p1, p2 } => todo!(),
            Sdf2dKind::EquTriangle { radius } => todo!(),
            Sdf2dKind::IsoTriangle { height, half_base } => todo!(),
            Sdf2dKind::Pentagon { radius } => todo!(),
            Sdf2dKind::Hexagon { radius } => todo!(),
            Sdf2dKind::Octogon { radius } => todo!(),
            Sdf2dKind::Pentragram { radius } => todo!(),
            Sdf2dKind::Hexagram { radius } => todo!(),
            Sdf2dKind::Star {
                outer_radius,
                inner_radius_factor,
                point_count,
            } => todo!(),
            Sdf2dKind::Pie { open_angle, radius } => todo!(),
            Sdf2dKind::CutDisk { radius, cut_height } => todo!(),
            Sdf2dKind::Arc {
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2dKind::Ring {
                normal,
                radius,
                thickness,
            } => todo!(),
            Sdf2dKind::Horseshoe {
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Sdf2dKind::Vesica { width, height } => todo!(),
            Sdf2dKind::Moon {
                center_distance,
                outer_radius,
                inner_radius,
            } => todo!(),
            Sdf2dKind::Egg {
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => todo!(),
            Sdf2dKind::Heart => todo!(),
            Sdf2dKind::RoundedX {
                thickness,
                corner_radius,
            } => todo!(),
            Sdf2dKind::Cross {
                arm_half_extent,
                corner_radius,
            } => todo!(),
            Sdf2dKind::CircleCross { arm_extent } => todo!(),
            Sdf2dKind::BlobbyCross { half_extent } => todo!(),
            Sdf2dKind::Parabola { curvature } => todo!(),
            Sdf2dKind::ParabolaSegment { half_width, height } => todo!(),
            Sdf2dKind::Hyperbola {
                curvature,
                half_length,
            } => todo!(),
            Sdf2dKind::QuadraticBezier {
                point_a,
                point_b,
                point_c,
            } => todo!(),
            Sdf2dKind::QuadraticCircle => todo!(),
            Sdf2dKind::Tunnel { half_extents } => todo!(),
            Sdf2dKind::Stairs { step_size, count } => todo!(),
            Sdf2dKind::CircleWave { wave_angle, radius } => todo!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sdf2dKind {
    Circle {
        radius: f32,
    },
    Ellipse {
        ab: Vec2,
    },
    Capsule {
        length: f32,
        radius: f32,
    },
    UnevenCapsule {
        length: f32,
        origin_radius: f32,
        tip_radius: f32,
    },
    Rect {
        half_size: Vec2,
    },
    RoundedRect {
        half_size: Vec2,
        corner_radii: Vec4,
    },
    ChamferRect {
        half_size: Vec2,
        chamfers: Vec4,
    },
    Rhombus {
        half_size: Vec2,
    },
    Trapezoid {
        bottom_radius: f32,
        top_radius: f32,
        half_height: f32,
    },
    Parallelogram {
        width: f32,
        height: f32,
        skew: f32,
    },
    Triangle {
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
    },
    EquTriangle {
        radius: f32,
    },
    IsoTriangle {
        height: f32,
        half_base: f32,
    },
    Pentagon {
        radius: f32,
    },
    Hexagon {
        radius: f32,
    },
    Octogon {
        radius: f32,
    },
    Pentragram {
        radius: f32,
    },
    Hexagram {
        radius: f32,
    },
    Star {
        outer_radius: f32,
        /// Must be between 2 and `point_count`
        inner_radius_factor: f32,
        point_count: u32,
    },
    Pie {
        open_angle: Vec2,
        radius: f32,
    },
    CutDisk {
        radius: f32,
        cut_height: f32,
    },
    Arc {
        open_angle: Vec2,
        radius: f32,
        thickness: f32,
    },
    Ring {
        normal: Vec2,
        radius: f32,
        thickness: f32,
    },
    Horseshoe {
        open_angle: Vec2,
        radius: f32,
        thickness: Vec2,
    },
    Vesica {
        width: f32,
        height: f32,
    },
    Moon {
        center_distance: f32,
        outer_radius: f32,
        inner_radius: f32,
    },
    Egg {
        half_height: f32,
        top_radius: f32,
        bottom_radius: f32,
        bulge: f32,
    },
    Heart,
    RoundedX {
        thickness: f32,
        corner_radius: f32,
    },
    Cross {
        arm_half_extent: Vec2,
        corner_radius: f32,
    },
    CircleCross {
        arm_extent: f32,
    },
    BlobbyCross {
        half_extent: f32,
    },
    Parabola {
        curvature: f32,
    },
    ParabolaSegment {
        half_width: f32,
        height: f32,
    },
    Hyperbola {
        curvature: f32,
        half_length: f32,
    },
    QuadraticBezier {
        point_a: Vec2,
        point_b: Vec2,
        point_c: Vec2,
    },
    QuadraticCircle,
    Tunnel {
        half_extents: Vec2,
    },
    Stairs {
        step_size: Vec2,
        count: f32,
    },
    CircleWave {
        wave_angle: f32,
        radius: f32,
    },
}

impl Sdf2dKind {
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius }
    }

    pub fn capsule(length: f32, radius: f32) -> Self {
        Self::Capsule { length, radius }
    }

    pub fn rect(half_size: impl Into<Vec2>) -> Self {
        Self::Rect {
            half_size: half_size.into(),
        }
    }

    pub fn rounded_rect(half_size: impl Into<Vec2>, corner_radii: impl Into<Vec4>) -> Self {
        Self::RoundedRect {
            half_size: half_size.into(),
            corner_radii: corner_radii.into(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Circle { .. } => "Circle",
            Self::Ellipse { .. } => "Ellipse",
            Self::Capsule { .. } => "Capsule",
            Self::UnevenCapsule { .. } => "UnevenCapsule",
            Self::Rect { .. } => "Rect",
            Self::RoundedRect { .. } => "RoundedRect",
            Self::ChamferRect { .. } => "ChamferRect",
            Self::Rhombus { .. } => "Rhombus",
            Self::Trapezoid { .. } => "Trapezoid",
            Self::Parallelogram { .. } => "Parallelogram",
            Self::Triangle { .. } => "Triangle",
            Self::EquTriangle { .. } => "EquTriangle",
            Self::IsoTriangle { .. } => "IsoTriangle",
            Self::Pentagon { .. } => "Pentagon",
            Self::Hexagon { .. } => "Hexagon",
            Self::Octogon { .. } => "Octogon",
            Self::Pentragram { .. } => "Pentragram",
            Self::Hexagram { .. } => "Hexagram",
            Self::Star { .. } => "Star",
            Self::Pie { .. } => "Pie",
            Self::CutDisk { .. } => "CutDisk",
            Self::Arc { .. } => "Arc",
            Self::Ring { .. } => "Ring",
            Self::Horseshoe { .. } => "Horseshoe",
            Self::Vesica { .. } => "Vesica",
            Self::Moon { .. } => "Moon",
            Self::Egg { .. } => "Egg",
            Self::Heart { .. } => "Heart",
            Self::RoundedX { .. } => "RoundedX",
            Self::Cross { .. } => "Cross",
            Self::CircleCross { .. } => "CircleCross",
            Self::BlobbyCross { .. } => "BobblyCross",
            Self::Parabola { .. } => "Parabola",
            Self::ParabolaSegment { .. } => "ParabolaSegment",
            Self::Hyperbola { .. } => "Hyperbola",
            Self::QuadraticBezier { .. } => "QuadraticBezier",
            Self::QuadraticCircle { .. } => "QuadraticCircle",
            Self::Tunnel { .. } => "Tunnel",
            Self::Stairs { .. } => "Stairs",
            Self::CircleWave { .. } => "CircleWave",
        }
    }

    /// Distance from shape, point mut be in local space
    pub fn dist(self, point: Vec2) -> f32 {
        match self {
            Self::Circle { radius } => circle_sdf_2d(point, radius),
            Self::Ellipse { ab } => todo!(),
            Self::Capsule { length, radius } => capsule_sdf_2d(point, radius, length),
            Self::UnevenCapsule {
                length,
                origin_radius,
                tip_radius,
            } => todo!(),
            Self::Rect { half_size, .. } => rect_sdf_2d(point, half_size),
            Self::RoundedRect {
                half_size,
                corner_radii,
            } => rounded_rect_sdf_2d(point, half_size, corner_radii),
            Self::ChamferRect {
                half_size,
                chamfers,
            } => todo!(),
            Self::Rhombus { half_size, .. } => todo!(),
            Self::Trapezoid {
                bottom_radius,
                top_radius,
                half_height,
            } => todo!(),
            Self::Parallelogram {
                width,
                height,
                skew,
            } => todo!(),
            Self::Triangle { p0, p1, p2, .. } => todo!(),
            Self::EquTriangle { radius, .. } => todo!(),
            Self::IsoTriangle {
                height, half_base, ..
            } => todo!(),
            Self::Pentagon { radius, .. } => todo!(),
            Self::Hexagon { radius, .. } => todo!(),
            Self::Octogon { radius, .. } => todo!(),
            Self::Pentragram { radius, .. } => todo!(),
            Self::Hexagram { radius, .. } => todo!(),
            Self::Star {
                outer_radius,
                inner_radius_factor,
                point_count,
            } => todo!(),
            Self::Pie {
                open_angle, radius, ..
            } => todo!(),
            Self::CutDisk {
                radius, cut_height, ..
            } => todo!(),
            Self::Arc {
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Self::Ring {
                normal,
                radius,
                thickness,
            } => todo!(),
            Self::Horseshoe {
                open_angle,
                radius,
                thickness,
            } => todo!(),
            Self::Vesica { width, height, .. } => todo!(),
            Self::Moon {
                center_distance,
                outer_radius,
                inner_radius,
            } => todo!(),
            Self::Egg {
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => todo!(),
            Self::Heart => todo!(),
            Self::RoundedX {
                thickness,
                corner_radius,
            } => todo!(),
            Self::Cross {
                arm_half_extent,
                corner_radius,
            } => todo!(),
            Self::CircleCross { arm_extent, .. } => todo!(),
            Self::BlobbyCross { half_extent, .. } => todo!(),
            Self::Parabola { curvature, .. } => todo!(),
            Self::ParabolaSegment {
                half_width, height, ..
            } => todo!(),
            Self::Hyperbola {
                curvature,
                half_length,
            } => todo!(),
            Self::QuadraticBezier {
                point_a,
                point_b,
                point_c,
            } => todo!(),
            Self::QuadraticCircle => todo!(),
            Self::Tunnel { half_extents, .. } => todo!(),
            Self::Stairs {
                step_size, count, ..
            } => todo!(),
            Self::CircleWave {
                wave_angle, radius, ..
            } => todo!(),
        }
    }
}

impl Default for Sdf2dKind {
    fn default() -> Self {
        Self::circle(1.)
    }
}

#[inline]
pub fn circle_sdf_2d(point: Vec2, radius: f32) -> f32 {
    point.length() - radius
}

#[inline]
pub fn capsule_sdf_2d(point: Vec2, radius: f32, length: f32) -> f32 {
    let h = point.x.clamp(0., length);
    (point - Vec2::new(h, 0.)).length() - radius
}

#[inline]
//REVIEW: See if this still will work if point is in local space now
pub fn capsule_endpoints_sdf_2d(point: Vec2, a: Vec2, b: Vec2, radius: f32) -> f32 {
    let ab = b - a;
    let h = ((point - a).dot(ab) / ab.dot(ab).max(1e-8)).clamp(0., 1.);
    (point - (a + ab * h)).length() - radius
}

#[inline]
pub fn rect_sdf_2d(point: Vec2, half_size: Vec2) -> f32 {
    let q = point.abs() - half_size;
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.)
}

#[inline]
pub fn rounded_rect_sdf_2d(point: Vec2, half_size: Vec2, corner_radii: Vec4) -> f32 {
    let mut r = if point.x > 0. {
        Vec2::new(corner_radii.x, corner_radii.y)
    } else {
        Vec2::new(corner_radii.z, corner_radii.w)
    };
    if point.y <= 0. {
        r.x = r.y;
    }
    let q = point.abs() - half_size + Vec2::from(r.x);
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
