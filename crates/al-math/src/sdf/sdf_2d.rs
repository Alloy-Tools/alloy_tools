//! Gathered from `https://iquilezles.org/articles/distfunctions2d/`
use crate::{
    sdf::Sdf2dPrim,
    transform::Transform2d,
    vec::{Vec2, Vec4},
};

/// GLSL/WGSL style sign that returns zero at the origin.
#[inline]
fn sign(value: f32) -> f32 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// GLSL style `mod`, which is always non-negative for a positive modulus.
#[inline]
fn mod_(value: f32, modulus: f32) -> f32 {
    value - modulus * (value / modulus).floor()
}

#[inline]
fn dot2(value: Vec2) -> f32 {
    value.length_squared()
}

#[inline]
fn v2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dOp {
    round: f32,
    onion: f32,
    elongate: Vec2,
}

pub const SDF_OP_SIZE: usize = std::mem::size_of::<Sdf2dOp>();

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
        let (local, inv_det_sqrt) = match transform.try_inverse() {
            Some(inv) => (
                inv.transform_point(point),
                //REVIEW: Want to use uniform scale for collision, should I use `.cols[0].length()` or `.determinant().abs().sqrt()`
                inv.linear.determinant().abs().sqrt(),
            ),
            None => (point, 1.),
        };
        // elongate
        let q = local - local.clamp(-ops.elongate, ops.elongate);
        let mut d = self.shape.dist(q);

        if ops.onion > 0. {
            d = d.abs() - ops.onion;
        }
        d = d - ops.round;

        d / inv_det_sqrt
    }

    pub fn to_prim(self, world: Transform2d, color: Vec4) -> Sdf2dPrim {
        let composed = world.compose(&self.transform);
        let m_inv = composed
            .linear
            .try_inverse()
            .unwrap_or(crate::matrix::Mat2::IDENTITY)
            .to_cols_array();
        let center = composed.translation;
        match self.shape {
            Sdf2dKind::Circle { radius } => Sdf2dPrim::circle(center, radius, m_inv, color),
            Sdf2dKind::Ellipse { ab } => Sdf2dPrim::ellipse(center, ab, m_inv, color),
            Sdf2dKind::Capsule { length, radius } => {
                Sdf2dPrim::capsule(center, length, radius, m_inv, color)
            }
            Sdf2dKind::UnevenCapsule {
                length,
                origin_radius,
                tip_radius,
            } => Sdf2dPrim::uneven_capsule(center, length, origin_radius, tip_radius, m_inv, color),
            Sdf2dKind::Rect { half_size } => Sdf2dPrim::rect(center, half_size, m_inv, color),
            Sdf2dKind::RoundedRect {
                half_size,
                corner_radii,
            } => Sdf2dPrim::rounded_rect(center, half_size, corner_radii, m_inv, color),
            Sdf2dKind::ChamferRect {
                half_size,
                chamfers,
            } => Sdf2dPrim::chamfer_rect(center, half_size, chamfers, m_inv, color),
            Sdf2dKind::Rhombus { half_size } => Sdf2dPrim::rhombus(center, half_size, m_inv, color),
            Sdf2dKind::Trapezoid {
                bottom_radius,
                top_radius,
                half_height,
            } => Sdf2dPrim::trapezoid(center, bottom_radius, top_radius, half_height, m_inv, color),
            Sdf2dKind::Parallelogram {
                width,
                height,
                skew,
            } => Sdf2dPrim::parallelogram(center, width, height, skew, m_inv, color),
            Sdf2dKind::Triangle { p0, p1, p2 } => {
                Sdf2dPrim::triangle(center, p0, p1, p2, m_inv, color)
            }
            Sdf2dKind::EquTriangle { radius } => {
                Sdf2dPrim::equ_triangle(center, radius, m_inv, color)
            }
            Sdf2dKind::IsoTriangle { height, half_base } => {
                Sdf2dPrim::iso_triangle(center, height, half_base, m_inv, color)
            }
            Sdf2dKind::Pentagon { radius } => Sdf2dPrim::pentagon(center, radius, m_inv, color),
            Sdf2dKind::Hexagon { radius } => Sdf2dPrim::hexagon(center, radius, m_inv, color),
            Sdf2dKind::Octogon { radius } => Sdf2dPrim::octogon(center, radius, m_inv, color),
            Sdf2dKind::Pentagram { radius } => Sdf2dPrim::pentagram(center, radius, m_inv, color),
            Sdf2dKind::Hexagram { radius } => Sdf2dPrim::hexagram(center, radius, m_inv, color),
            Sdf2dKind::Star {
                outer_radius,
                inner_radius_factor,
                point_count,
            } => Sdf2dPrim::star(
                center,
                outer_radius,
                inner_radius_factor,
                point_count,
                m_inv,
                color,
            ),
            Sdf2dKind::Pie { open_angle, radius } => {
                Sdf2dPrim::pie(center, open_angle, radius, m_inv, color)
            }
            Sdf2dKind::CutDisk { radius, cut_height } => {
                Sdf2dPrim::cut_disk(center, radius, cut_height, m_inv, color)
            }
            Sdf2dKind::Arc {
                open_angle,
                radius,
                thickness,
            } => Sdf2dPrim::arc(center, open_angle, radius, thickness, m_inv, color),
            Sdf2dKind::Ring {
                normal,
                radius,
                thickness,
            } => Sdf2dPrim::ring(center, normal, radius, thickness, m_inv, color),
            Sdf2dKind::Horseshoe {
                open_angle,
                radius,
                thickness,
            } => Sdf2dPrim::horseshoe(center, open_angle, radius, thickness, m_inv, color),
            Sdf2dKind::Vesica { width, height } => {
                Sdf2dPrim::vesica(center, width, height, m_inv, color)
            }
            Sdf2dKind::Moon {
                center_distance,
                outer_radius,
                inner_radius,
            } => Sdf2dPrim::moon(
                center,
                center_distance,
                outer_radius,
                inner_radius,
                m_inv,
                color,
            ),
            Sdf2dKind::Egg {
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => Sdf2dPrim::egg(
                center,
                half_height,
                top_radius,
                bottom_radius,
                bulge,
                m_inv,
                color,
            ),
            Sdf2dKind::Heart => Sdf2dPrim::heart(center, m_inv, color),
            Sdf2dKind::RoundedX {
                thickness,
                corner_radius,
            } => Sdf2dPrim::rounded_x(center, thickness, corner_radius, m_inv, color),
            Sdf2dKind::Cross {
                arm_half_extent,
                corner_radius,
            } => Sdf2dPrim::cross(center, arm_half_extent, corner_radius, m_inv, color),
            Sdf2dKind::CircleCross { arm_extent } => {
                Sdf2dPrim::circle_cross(center, arm_extent, m_inv, color)
            }
            Sdf2dKind::BlobbyCross { half_extent } => {
                Sdf2dPrim::blobby_cross(center, half_extent, m_inv, color)
            }
            Sdf2dKind::Parabola { curvature } => {
                Sdf2dPrim::parabola(center, curvature, m_inv, color)
            }
            Sdf2dKind::ParabolaSegment { half_width, height } => {
                Sdf2dPrim::parabola_segment(center, half_width, height, m_inv, color)
            }
            Sdf2dKind::Hyperbola {
                curvature,
                half_length,
            } => Sdf2dPrim::hyperbola(center, curvature, half_length, m_inv, color),
            Sdf2dKind::QuadraticBezier {
                point_a,
                point_b,
                point_c,
            } => Sdf2dPrim::quadratic_bezier(center, point_a, point_b, point_c, m_inv, color),
            Sdf2dKind::QuadraticCircle => Sdf2dPrim::quadratic_circle(center, m_inv, color),
            Sdf2dKind::Tunnel { half_extents } => {
                Sdf2dPrim::tunnel(center, half_extents, m_inv, color)
            }
            Sdf2dKind::Stairs { step_size, count } => {
                Sdf2dPrim::stairs(center, step_size, count, m_inv, color)
            }
            Sdf2dKind::CircleWave { wave_angle, radius } => {
                Sdf2dPrim::circle_wave(center, wave_angle, radius, m_inv, color)
            }
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
    Pentagram {
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
            Self::Pentagram { .. } => "Pentagram",
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
            Self::BlobbyCross { .. } => "BlobbyCross",
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

    /// Distance from shape, point must be in local space.
    pub fn dist(self, point: Vec2) -> f32 {
        match self {
            Self::Circle { radius } => circle_sdf_2d(point, radius),
            Self::Ellipse { ab } => ellipse_sdf_2d(point, ab),
            Self::Capsule { length, radius } => capsule_sdf_2d(point, radius, length),
            Self::UnevenCapsule {
                length,
                origin_radius,
                tip_radius,
            } => uneven_capsule_sdf_2d(point, origin_radius, tip_radius, length),
            Self::Rect { half_size } => rect_sdf_2d(point, half_size),
            Self::RoundedRect {
                half_size,
                corner_radii,
            } => rounded_rect_sdf_2d(point, half_size, corner_radii),
            Self::ChamferRect {
                half_size,
                chamfers,
            } => chamfer_rect_sdf_2d(point, half_size, chamfers),
            Self::Rhombus { half_size } => rhombus_sdf_2d(point, half_size),
            Self::Trapezoid {
                bottom_radius,
                top_radius,
                half_height,
            } => trapezoid_sdf_2d(point, bottom_radius, top_radius, half_height),
            Self::Parallelogram {
                width,
                height,
                skew,
            } => parallelogram_sdf_2d(point, width, height, skew),
            Self::Triangle { p0, p1, p2 } => triangle_sdf_2d(point, p0, p1, p2),
            Self::EquTriangle { radius } => equ_triangle_sdf_2d(point, radius),
            Self::IsoTriangle { height, half_base } => {
                iso_triangle_sdf_2d(point, height, half_base)
            }
            Self::Pentagon { radius } => pentagon_sdf_2d(point, radius),
            Self::Hexagon { radius } => hexagon_sdf_2d(point, radius),
            Self::Octogon { radius } => octogon_sdf_2d(point, radius),
            Self::Pentagram { radius } => pentagram_sdf_2d(point, radius),
            Self::Hexagram { radius } => hexagram_sdf_2d(point, radius),
            Self::Star {
                outer_radius,
                inner_radius_factor,
                point_count,
            } => star_sdf_2d(point, outer_radius, inner_radius_factor, point_count),
            Self::Pie { open_angle, radius } => pie_sdf_2d(point, open_angle, radius),
            Self::CutDisk { radius, cut_height } => cut_disk_sdf_2d(point, radius, cut_height),
            Self::Arc {
                open_angle,
                radius,
                thickness,
            } => arc_sdf_2d(point, open_angle, radius, thickness),
            Self::Ring {
                normal,
                radius,
                thickness,
            } => ring_sdf_2d(point, normal, radius, thickness),
            Self::Horseshoe {
                open_angle,
                radius,
                thickness,
            } => horseshoe_sdf_2d(point, open_angle, radius, thickness),
            Self::Vesica { width, height } => vesica_sdf_2d(point, width, height),
            Self::Moon {
                center_distance,
                outer_radius,
                inner_radius,
            } => moon_sdf_2d(point, center_distance, outer_radius, inner_radius),
            Self::Egg {
                half_height,
                top_radius,
                bottom_radius,
                bulge,
            } => egg_sdf_2d(point, half_height, top_radius, bottom_radius, bulge),
            Self::Heart => heart_sdf_2d(point),
            Self::RoundedX {
                thickness,
                corner_radius,
            } => rounded_x_sdf_2d(point, thickness, corner_radius),
            Self::Cross {
                arm_half_extent,
                corner_radius,
            } => cross_sdf_2d(point, arm_half_extent, corner_radius),
            Self::CircleCross { arm_extent } => circle_cross_sdf_2d(point, arm_extent),
            Self::BlobbyCross { half_extent } => blobby_cross_sdf_2d(point, half_extent),
            Self::Parabola { curvature } => parabola_sdf_2d(point, curvature),
            Self::ParabolaSegment { half_width, height } => {
                parabola_segment_sdf_2d(point, half_width, height)
            }
            Self::Hyperbola {
                curvature,
                half_length,
            } => hyperbola_sdf_2d(point, curvature, half_length),
            Self::QuadraticBezier {
                point_a,
                point_b,
                point_c,
            } => quadratic_bezier_sdf_2d(point, point_a, point_b, point_c),
            Self::QuadraticCircle => quadratic_circle_sdf_2d(point),
            Self::Tunnel { half_extents } => tunnel_sdf_2d(point, half_extents),
            Self::Stairs { step_size, count } => stairs_sdf_2d(point, step_size, count),
            Self::CircleWave { wave_angle, radius } => {
                circle_wave_sdf_2d(point, wave_angle, radius)
            }
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
pub fn ellipse_sdf_2d(point: Vec2, ab: Vec2) -> f32 {
    let mut p = point.abs();
    let mut ab = ab;
    if p.x > p.y {
        p = v2(p.y, p.x);
        ab = v2(ab.y, ab.x);
    }

    let l = ab.y * ab.y - ab.x * ab.x;
    if l.abs() < f32::EPSILON {
        return p.length() - ab.x.max(ab.y);
    }

    let m = ab.x * p.x / l;
    let m2 = m * m;
    let n = ab.y * p.y / l;
    let n2 = n * n;
    let c = (m2 + n2 - 1.0) / 3.0;
    let c3 = c * c * c;
    let q = c3 + m2 * n2 * 2.0;
    let d = c3 + m2 * n2;
    let g = m + m * n2;

    let co = if d < 0.0 {
        let h = (q / c3).acos() / 3.0;
        let s = h.cos();
        let t = h.sin() * 3.0f32.sqrt();
        let rx = (-c * (s + t + 2.0) + m2).sqrt();
        let ry = (-c * (s - t + 2.0) + m2).sqrt();
        (ry + sign(l) * rx + g.abs() / (rx * ry) - m) / 2.0
    } else {
        let h = 2.0 * m * n * d.sqrt();
        let s = sign(q + h) * (q + h).abs().powf(1.0 / 3.0);
        let u = sign(q - h) * (q - h).abs().powf(1.0 / 3.0);
        let rx = -s - u - c * 4.0 + 2.0 * m2;
        let ry = (s - u) * 3.0f32.sqrt();
        let rm = (rx * rx + ry * ry).sqrt();
        (ry / (rm - rx).sqrt() + 2.0 * g / rm - m) / 2.0
    };

    let r = v2(ab.x * co, ab.y * (1.0 - co * co).max(0.0).sqrt());
    (r - p).length() * sign(p.y - r.y)
}

#[inline]
pub fn capsule_sdf_2d(point: Vec2, radius: f32, length: f32) -> f32 {
    let h = point.x.clamp(0., length);
    (point - v2(h, 0.)).length() - radius
}

#[inline]
pub fn capsule_endpoints_sdf_2d(point: Vec2, a: Vec2, b: Vec2, radius: f32) -> f32 {
    let ab = b - a;
    let h = ((point - a).dot(ab) / ab.dot(ab).max(1e-8)).clamp(0., 1.);
    (point - (a + ab * h)).length() - radius
}

#[inline]
pub fn uneven_capsule_sdf_2d(point: Vec2, origin_radius: f32, tip_radius: f32, length: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    let b = (origin_radius - tip_radius) / length;
    let a = (1.0 - b * b).max(0.0).sqrt();
    let k = p.dot(v2(-b, a));
    if k < 0.0 {
        return p.length() - origin_radius;
    }
    if k > a * length {
        return (p - v2(0.0, length)).length() - tip_radius;
    }
    p.dot(v2(a, b)) - origin_radius
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
    q.x.max(q.y).min(0.) + q.max(Vec2::ZERO).length() - r.x
}

#[inline]
pub fn chamfer_rect_sdf_2d(point: Vec2, half_size: Vec2, chamfers: Vec4) -> f32 {
    let chamfer = if point.x > 0.0 {
        if point.y > 0.0 {
            chamfers.x
        } else {
            chamfers.y
        }
    } else if point.y > 0.0 {
        chamfers.z
    } else {
        chamfers.w
    };

    let mut p = point.abs() - half_size;
    if p.y > p.x {
        p = v2(p.y, p.x);
    }
    p.y += chamfer;

    const K: f32 = 1.0 - std::f32::consts::SQRT_2;
    if p.y < 0.0 && p.y + p.x * K < 0.0 {
        return p.x;
    }
    if p.x < p.y {
        return (p.x + p.y) * std::f32::consts::FRAC_1_SQRT_2;
    }
    p.length()
}

#[inline]
pub fn rhombus_sdf_2d(point: Vec2, half_size: Vec2) -> f32 {
    let b = v2(half_size.x, -half_size.y);
    let mut p = point.abs();
    let h = ((b.dot(p) + b.y * b.y) / b.dot(b)).clamp(0.0, 1.0);
    p -= b * v2(h, h - 1.0);
    p.length() * sign(p.x)
}

#[inline]
pub fn trapezoid_sdf_2d(point: Vec2, bottom_radius: f32, top_radius: f32, half_height: f32) -> f32 {
    let k1 = v2(top_radius, half_height);
    let k2 = v2(top_radius - bottom_radius, 2.0 * half_height);

    let mut p = point;
    p.x = p.x.abs();

    let nearest_radius = if p.y < 0.0 { bottom_radius } else { top_radius };
    let ca = v2(p.x - p.x.min(nearest_radius), p.y.abs() - half_height);
    let cb = p - k1 + k2 * ((k1 - p).dot(k2) / dot2(k2)).clamp(0.0, 1.0);

    let s = if cb.x < 0.0 && ca.y < 0.0 { -1.0 } else { 1.0 };
    s * dot2(ca).min(dot2(cb)).sqrt()
}

#[inline]
pub fn parallelogram_sdf_2d(point: Vec2, width: f32, height: f32, skew: f32) -> f32 {
    let e = v2(skew, height);
    let p = if point.y < 0.0 { -point } else { point };
    let mut w = p - e;
    w.x -= w.x.clamp(-width, width);
    let mut d = v2(dot2(w), -w.y);

    let s = p.x * e.y - p.y * e.x;
    let p2 = if s < 0.0 { -p } else { p };
    let mut v = p2 - v2(width, 0.0);
    v -= e * (v.dot(e) / dot2(e)).clamp(-1.0, 1.0);
    d = d.min(v2(dot2(v), width * height - s.abs()));

    d.x.sqrt() * sign(-d.y)
}

#[inline]
pub fn triangle_sdf_2d(point: Vec2, p0: Vec2, p1: Vec2, p2: Vec2) -> f32 {
    let e0 = p1 - p0;
    let e1 = p2 - p1;
    let e2 = p0 - p2;
    let v0 = point - p0;
    let v1 = point - p1;
    let v2_ = point - p2;

    let pq0 = v0 - e0 * (v0.dot(e0) / dot2(e0)).clamp(0.0, 1.0);
    let pq1 = v1 - e1 * (v1.dot(e1) / dot2(e1)).clamp(0.0, 1.0);
    let pq2 = v2_ - e2 * (v2_.dot(e2) / dot2(e2)).clamp(0.0, 1.0);

    let s = sign(e0.x * e2.y - e0.y * e2.x);
    let d = v2(dot2(pq0), s * (v0.x * e0.y - v0.y * e0.x))
        .min(v2(dot2(pq1), s * (v1.x * e1.y - v1.y * e1.x)))
        .min(v2(dot2(pq2), s * (v2_.x * e2.y - v2_.y * e2.x)));

    -d.x.sqrt() * sign(d.y)
}

#[inline]
pub fn equ_triangle_sdf_2d(point: Vec2, radius: f32) -> f32 {
    let k = 3.0f32.sqrt();

    let mut p = point;
    p.x = p.x.abs() - radius;
    p.y += radius / k;
    if p.x + k * p.y > 0.0 {
        let nx = p.x - k * p.y;
        let ny = -k * p.x - p.y;
        p = v2(nx, ny) / 2.0;
    }
    p.x -= p.x.clamp(-2.0 * radius, 0.0);
    -p.length() * sign(p.y)
}

#[inline]
pub fn iso_triangle_sdf_2d(point: Vec2, height: f32, half_base: f32) -> f32 {
    let q = v2(half_base, height);
    let mut p = point;
    p.x = p.x.abs();

    let a = p - q * (p.dot(q) / dot2(q)).clamp(0.0, 1.0);
    let b = p - q * v2((p.x / q.x).clamp(0.0, 1.0), 1.0);
    let s = -sign(q.y);
    let d = v2(dot2(a), s * (p.x * q.y - p.y * q.x)).min(v2(dot2(b), s * (p.y - q.y)));

    -d.x.sqrt() * sign(d.y)
}

#[inline]
pub fn pentagon_sdf_2d(point: Vec2, radius: f32) -> f32 {
    let k = v2(0.809016994, 0.587785252);
    let k_z = 0.726542528;

    let mut p = point;
    p.x = p.x.abs();
    p -= 2.0 * p.dot(v2(-k.x, k.y)).min(0.0) * v2(-k.x, k.y);
    p -= 2.0 * p.dot(v2(k.x, k.y)).min(0.0) * v2(k.x, k.y);
    p -= v2(p.x.clamp(-radius * k_z, radius * k_z), radius);
    p.length() * sign(p.y)
}

#[inline]
pub fn hexagon_sdf_2d(point: Vec2, radius: f32) -> f32 {
    let k = v2(-0.866025404, 0.5);
    let k_z = 0.577350269;

    let mut p = point.abs();
    p -= 2.0 * p.dot(k).min(0.0) * k;
    p -= v2(p.x.clamp(-radius * k_z, radius * k_z), radius);
    p.length() * sign(p.y)
}

#[inline]
pub fn octogon_sdf_2d(point: Vec2, radius: f32) -> f32 {
    let k = v2(-0.9238795325, 0.3826834323);
    let k_z = 0.4142135623;

    let mut p = point.abs();
    p -= 2.0 * p.dot(k).min(0.0) * k;
    p -= 2.0 * p.dot(v2(-k.x, k.y)).min(0.0) * v2(-k.x, k.y);
    p -= v2(p.x.clamp(-radius * k_z, radius * k_z), radius);
    p.length() * sign(p.y)
}

#[inline]
pub fn pentagram_sdf_2d(point: Vec2, radius: f32) -> f32 {
    const K1X: f32 = 0.809016994;
    const K2X: f32 = 0.309016994;
    const K1Y: f32 = 0.587785252;
    const K2Y: f32 = 0.951056516;
    const K1Z: f32 = 0.726542528;

    let v1 = v2(K1X, -K1Y);
    let v2_ = v2(-K1X, -K1Y);
    let v3 = v2(K2X, -K2Y);

    let mut p = point;
    p.x = p.x.abs();
    p -= 2.0 * p.dot(v1).max(0.0) * v1;
    p -= 2.0 * p.dot(v2_).max(0.0) * v2_;
    p.x = p.x.abs();
    p.y -= radius;

    (p - v3 * p.dot(v3).clamp(0.0, K1Z * radius)).length() * sign(p.y * v3.x - p.x * v3.y)
}

#[inline]
pub fn hexagram_sdf_2d(point: Vec2, radius: f32) -> f32 {
    let k = Vec4::new(-0.5, 0.8660254038, 0.5773502692, 1.7320508076);

    let mut p = point.abs();
    p -= 2.0 * p.dot(v2(k.x, k.y)).min(0.0) * v2(k.x, k.y);
    p -= 2.0 * p.dot(v2(k.y, k.x)).min(0.0) * v2(k.y, k.x);
    p -= v2(p.x.clamp(radius * k.z, radius * k.w), radius);
    p.length() * sign(p.y)
}

#[inline]
pub fn star_sdf_2d(
    point: Vec2,
    outer_radius: f32,
    inner_radius_factor: f32,
    point_count: u32,
) -> f32 {
    // Precomputable per shape.
    let an = std::f32::consts::PI / point_count as f32;
    let en = std::f32::consts::PI / inner_radius_factor;
    let acs = v2(an.cos(), an.sin());
    let ecs = v2(en.cos(), en.sin());

    let bn = mod_(point.x.atan2(point.y), 2.0 * an) - an;
    let mut p = point.length() * v2(bn.cos(), bn.sin().abs());
    p -= outer_radius * acs;
    p += ecs * (-p.dot(ecs)).clamp(0.0, outer_radius * acs.y / ecs.y);
    p.length() * sign(p.x)
}

#[inline]
pub fn pie_sdf_2d(point: Vec2, open_angle: Vec2, radius: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    let l = p.length() - radius;
    let m = (p - open_angle * p.dot(open_angle).clamp(0.0, radius)).length();
    l.max(m * sign(open_angle.y * p.x - open_angle.x * p.y))
}

#[inline]
pub fn cut_disk_sdf_2d(point: Vec2, radius: f32, cut_height: f32) -> f32 {
    let w = (radius * radius - cut_height * cut_height).max(0.0).sqrt();
    let mut p = point;
    p.x = p.x.abs();
    let s = ((cut_height - radius) * p.x * p.x + w * w * (cut_height + radius - 2.0 * p.y))
        .max(cut_height * p.x - w * p.y);

    if s < 0.0 {
        p.length() - radius
    } else if p.x < w {
        cut_height - p.y
    } else {
        (p - v2(w, cut_height)).length()
    }
}

#[inline]
pub fn arc_sdf_2d(point: Vec2, open_angle: Vec2, radius: f32, thickness: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    (if open_angle.y * p.x > open_angle.x * p.y {
        (p - open_angle * radius).length()
    } else {
        (p.length() - radius).abs()
    }) - thickness
}

#[inline]
pub fn ring_sdf_2d(point: Vec2, normal: Vec2, radius: f32, thickness: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    p = v2(
        normal.x * p.x - normal.y * p.y,
        normal.y * p.x + normal.x * p.y,
    );

    let half_thickness = thickness * 0.5;
    let a = (p.length() - radius).abs() - half_thickness;
    let b = v2(p.x, (radius - p.y).abs().max(0.0) - half_thickness).length() * sign(p.x);
    a.max(b)
}

#[inline]
pub fn horseshoe_sdf_2d(point: Vec2, open_angle: Vec2, radius: f32, thickness: Vec2) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    let l = p.length();
    p = v2(
        -open_angle.x * p.x + open_angle.y * p.y,
        open_angle.y * p.x + open_angle.x * p.y,
    );
    p = v2(
        if p.y > 0.0 || p.x > 0.0 {
            p.x
        } else {
            l * sign(-open_angle.x)
        },
        if p.x > 0.0 { p.y } else { l },
    );
    p = v2(p.x, (p.y - radius).abs()) - thickness;
    p.max(Vec2::ZERO).length() + p.x.max(p.y).min(0.0)
}

#[inline]
pub fn vesica_sdf_2d(point: Vec2, width: f32, height: f32) -> f32 {
    let d = 0.5 * (width * width - height * height) / height;
    let p = point.abs();
    let (center, radius) = if width * p.y < d * (p.x - width) {
        (v2(width, 0.0), 0.0)
    } else {
        (v2(0.0, -d), d + height)
    };
    (p - center).length() - radius
}

#[inline]
pub fn moon_sdf_2d(point: Vec2, center_distance: f32, outer_radius: f32, inner_radius: f32) -> f32 {
    let mut p = point;
    p.y = p.y.abs();

    let a = (outer_radius * outer_radius - inner_radius * inner_radius
        + center_distance * center_distance)
        / (2.0 * center_distance);
    let b = (outer_radius * outer_radius - a * a).max(0.0).sqrt();

    if center_distance * (p.x * b - p.y * a)
        > center_distance * center_distance * (b - p.y).max(0.0)
    {
        (p - v2(a, b)).length()
    } else {
        (p.length() - outer_radius).max(-((p - v2(center_distance, 0.0)).length() - inner_radius))
    }
}

#[inline]
pub fn egg_sdf_2d(
    point: Vec2,
    half_height: f32,
    top_radius: f32,
    bottom_radius: f32,
    bulge: f32,
) -> f32 {
    // Inigo's ra is the bottom radius and rb the top radius.
    let ra = bottom_radius;
    let rb = top_radius;
    let r = 0.5 * (half_height + ra + rb) / bulge;
    let da = r - ra;
    let db = r - rb;
    let y = (db * db - da * da - half_height * half_height) / (2.0 * half_height);
    let x = (da * da - y * y).max(0.0).sqrt();

    let mut p = point;
    p.x = p.x.abs();
    let k = p.y * x - p.x * y;
    if k > 0.0 && k < half_height * (p.x + x) {
        (p + v2(x, y)).length() - r
    } else {
        (p.length() - ra).min((v2(p.x, p.y - half_height)).length() - rb)
    }
}

#[inline]
pub fn heart_sdf_2d(point: Vec2) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    if p.y + p.x > 1.0 {
        return (p - v2(0.25, 0.75)).length() - std::f32::consts::SQRT_2 / 4.0;
    }
    let center = Vec2::from(0.5 * (p.x + p.y).max(0.0));
    dot2(p - v2(0.0, 1.0)).min(dot2(p - center)).sqrt() * sign(p.x - p.y)
}

#[inline]
pub fn rounded_x_sdf_2d(point: Vec2, thickness: f32, corner_radius: f32) -> f32 {
    let p = point.abs();
    (p - Vec2::from((p.x + p.y).min(thickness) * 0.5)).length() - corner_radius
}

#[inline]
pub fn cross_sdf_2d(point: Vec2, arm_half_extent: Vec2, corner_radius: f32) -> f32 {
    let mut p = point.abs();
    if p.y > p.x {
        p = v2(p.y, p.x);
    }
    let q = p - arm_half_extent;
    let k = q.y.max(q.x);
    let w = if k > 0.0 {
        q
    } else {
        v2(arm_half_extent.y - p.x, -k)
    };
    sign(k) * w.max(Vec2::ZERO).length() + corner_radius
}

#[inline]
pub fn circle_cross_sdf_2d(point: Vec2, arm_extent: f32) -> f32 {
    let k = 0.5 * (arm_extent + 1.0 / arm_extent);
    let p = point.abs();
    if p.x < 1.0 && p.y < p.x * (k - arm_extent) + arm_extent {
        k - (p - v2(1.0, k)).length()
    } else {
        dot2(p - v2(0.0, arm_extent))
            .min(dot2(p - v2(1.0, 0.0)))
            .sqrt()
    }
}

#[inline]
pub fn blobby_cross_sdf_2d(point: Vec2, half_extent: f32) -> f32 {
    let p = point.abs();
    let p = v2((p.x - p.y).abs(), 1.0 - p.x - p.y) / std::f32::consts::SQRT_2;

    let q = (half_extent - p.y - 0.25 / half_extent) / (6.0 * half_extent);
    let r = p.x / (half_extent * half_extent * 16.0);
    let h = r * r - q * q * q;

    let mut x = if h > 0.0 {
        let s = h.sqrt();
        (r + s).powf(1.0 / 3.0) - (r - s).abs().powf(1.0 / 3.0) * sign(s - r)
    } else {
        let s = q.sqrt();
        2.0 * s * ((r / (q * s)).acos() / 3.0).cos()
    };
    x = x.min(std::f32::consts::FRAC_1_SQRT_2);

    let z = v2(x, half_extent * (1.0 - 2.0 * x * x)) - p;
    z.length() * sign(z.y)
}

#[inline]
pub fn parabola_sdf_2d(point: Vec2, curvature: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();

    let ik = 1.0 / curvature;
    let p_ = ik * (p.y - 0.5 * ik) / 3.0;
    let q = 0.25 * ik * ik * p.x;
    let h = q * q - p_ * p_ * p_;
    let x = if h > 0.0 {
        let r = (q + h.sqrt()).powf(1.0 / 3.0);
        r + p_ / r
    } else {
        let r = p_.sqrt();
        2.0 * r * ((q / (p_ * r)).acos() / 3.0).cos()
    };

    (p - v2(x, curvature * x * x)).length() * sign(p.x - x)
}

#[inline]
pub fn parabola_segment_sdf_2d(point: Vec2, half_width: f32, height: f32) -> f32 {
    let mut p = point;
    p.x = p.x.abs();

    let ik = half_width * half_width / height;
    let p_ = ik * (height - p.y - 0.5 * ik) / 3.0;
    let q = p.x * ik * ik / 4.0;
    let h = q * q - p_ * p_ * p_;
    let x = if h > 0.0 {
        let r = (q + h.sqrt()).powf(1.0 / 3.0);
        r + p_ / r
    } else {
        let r = p_.sqrt();
        2.0 * r * ((q / (p_ * r)).acos() / 3.0).cos()
    };
    let x = x.min(half_width);

    (p - v2(x, height - x * x / ik)).length() * sign(ik * (p.y - height) + p.x * p.x)
}

#[inline]
pub fn hyperbola_sdf_2d(point: Vec2, curvature: f32, half_length: f32) -> f32 {
    let p = point.abs();
    let p = v2(p.x - p.y, p.x + p.y) / std::f32::consts::SQRT_2;

    let x2 = p.x * p.x / 16.0;
    let y2 = p.y * p.y / 16.0;
    let r = curvature * (4.0 * curvature - p.x * p.y) / 12.0;
    let q = (x2 - y2) * curvature * curvature;
    let h = q * q + r * r * r;

    let u = if h < 0.0 {
        let m = (-r).sqrt();
        m * ((q / (r * m)).acos() / 3.0).cos()
    } else {
        let m = (h.sqrt() - q).powf(1.0 / 3.0);
        (m - r / m) / 2.0
    };

    let w = (u + x2).sqrt();
    let b = curvature * p.y - x2 * p.x * 2.0;
    let t = (p.x / 4.0 - w + (2.0 * x2 - u + b / w / 4.0).sqrt()).max(
        (half_length * half_length * 0.5 + curvature).sqrt()
            - half_length / std::f32::consts::SQRT_2,
    );
    let d = (p - v2(t, curvature / t)).length();

    if p.x * p.y < curvature {
        d
    } else {
        -d
    }
}

#[inline]
pub fn quadratic_bezier_sdf_2d(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let a_ = b - a;
    let b_ = a - 2.0 * b + c;
    let c_ = a_ * 2.0;
    let d = a - point;

    let kk = 1.0 / dot2(b_);
    let kx = kk * a_.dot(b_);
    let ky = kk * (2.0 * dot2(a_) + d.dot(b_)) / 3.0;
    let kz = kk * d.dot(a_);

    let p = ky - kx * kx;
    let p3 = p * p * p;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    let h = q * q + 4.0 * p3;

    let res = if h >= 0.0 {
        let h = h.sqrt();
        let x = (v2(h, -h) - Vec2::from(q)) / 2.0;
        let uv = v2(
            sign(x.x) * x.x.abs().powf(1.0 / 3.0),
            sign(x.y) * x.y.abs().powf(1.0 / 3.0),
        );
        let t = (uv.x + uv.y - kx).clamp(0.0, 1.0);
        dot2(d + (c_ + b_ * t) * t)
    } else {
        let z = (-p).sqrt();
        let v = (q / (p * z * 2.0)).acos() / 3.0;
        let m = v.cos();
        let n = v.sin() * 1.732050808;
        let t0 = (m + m).mul_add(z, -kx).clamp(0.0, 1.0);
        let t1 = (-n - m).mul_add(z, -kx).clamp(0.0, 1.0);
        dot2(d + (c_ + b_ * t0) * t0).min(dot2(d + (c_ + b_ * t1) * t1))
    };

    res.sqrt()
}

#[inline]
pub fn quadratic_circle_sdf_2d(point: Vec2) -> f32 {
    let mut p = point.abs();
    if p.y > p.x {
        p = v2(p.y, p.x);
    }

    let a = p.x - p.y;
    let b = p.x + p.y;
    let c = (2.0 * b - 1.0) / 3.0;
    let h = a * a + c * c * c;

    let mut t = if h >= 0.0 {
        let h = h.sqrt();
        sign(h - a) * (h - a).abs().powf(1.0 / 3.0) - (h + a).powf(1.0 / 3.0)
    } else {
        let z = (-c).sqrt();
        let v = (a / (c * z)).acos() / 3.0;
        -z * (v.cos() + v.sin() * 1.732050808)
    };
    t *= 0.5;

    let w = v2(-t, t) + Vec2::from(0.75) - Vec2::from(t * t) - p;
    w.length() * sign(a * a * 0.5 + b - 1.5)
}

#[inline]
pub fn tunnel_sdf_2d(point: Vec2, half_extents: Vec2) -> f32 {
    let mut p = point;
    p.x = p.x.abs();
    p.y = -p.y;

    let mut q = p - half_extents;
    let d1 = dot2(v2(q.x.max(0.0), q.y));
    q.x = if p.y > 0.0 {
        q.x
    } else {
        p.length() - half_extents.x
    };
    let d2 = dot2(v2(q.x, q.y.max(0.0)));
    let d = d1.min(d2).sqrt();

    if q.x.max(q.y) < 0.0 {
        -d
    } else {
        d
    }
}

#[inline]
pub fn stairs_sdf_2d(point: Vec2, step_size: Vec2, count: f32) -> f32 {
    let ba = step_size * count;
    let mut p = point;

    let mut d =
        dot2(p - v2(p.x.clamp(0.0, ba.x), 0.0)).min(dot2(p - v2(ba.x, p.y.clamp(0.0, ba.y))));
    let mut s = sign((-p.y).max(p.x - ba.x));

    let dia = step_size.length();
    // mat2(wh.x, -wh.y, wh.y, wh.x)
    p = v2(
        step_size.x * p.x + step_size.y * p.y,
        -step_size.y * p.x + step_size.x * p.y,
    ) / dia;
    let id = (p.x / dia).round().clamp(0.0, count - 1.0);
    p.x -= id * dia;
    // mat2(wh.x, wh.y, -wh.y, wh.x)
    p = v2(
        step_size.x * p.x - step_size.y * p.y,
        step_size.y * p.x + step_size.x * p.y,
    ) / dia;

    let hh = step_size.y / 2.0;
    p.y -= hh;
    if p.y > hh * sign(p.x) {
        s = 1.0;
    }
    p = if id < 0.5 || p.x > 0.0 { p } else { -p };

    d = d.min(dot2(p - v2(0.0, p.y.clamp(-hh, hh))));
    d = d.min(dot2(p - v2(p.x.clamp(0.0, step_size.x), hh)));

    d.sqrt() * s
}

#[inline]
pub fn circle_wave_sdf_2d(point: Vec2, wave_angle: f32, radius: f32) -> f32 {
    let tb = 3.1415927 * 5.0 / 6.0 * wave_angle.max(0.0001);
    let co = v2(tb.sin(), tb.cos()) * radius;

    let mut p = point;
    p.x = (mod_(p.x, co.x * 4.0) - co.x * 2.0).abs();

    let p1 = p;
    let p2 = v2((p.x - 2.0 * co.x).abs(), -p.y + 2.0 * co.y);

    let d1 = if co.y * p1.x > co.x * p1.y {
        (p1 - co).length()
    } else {
        (p1.length() - radius).abs()
    };
    let d2 = if co.y * p2.x > co.x * p2.y {
        (p2 - co).length()
    } else {
        (p2.length() - radius).abs()
    };

    d1.min(d2)
}
