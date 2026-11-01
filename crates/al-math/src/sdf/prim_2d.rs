use crate::vec::{Vec2, Vec4};

pub const SDF_PRIM_SIZE: usize = std::mem::size_of::<Sdf2dPrim>();

/// GPU-side description of a single 2D SDF primitive.
///
/// The layout is mirrored by `SdfPrim` in `apps/tools/anim-tool/src/designer_sdf.wgsl`.
/// Shape parameters are packed into `a`, `b`, `c`, `radius` and `aux`; the exact
/// meaning of each slot is documented on the corresponding constructor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable,))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sdf2dPrim {
    a: Vec2,         // 8:  0 -7
    b: Vec2,         // 8:  8 -15
    radius: f32,     // 4:  16-19
    aux: f32,        // 4:  20-23
    aux_2: u32,      // 4:  24-27
    kind: u32,       // 4:  28-31
    c: Vec2,         // 8:  32-39
    _pad: Vec2,      // 8:  40-47 (keeps m_inv 16-byte aligned)
    m_inv: [f32; 4], // 16: 48-63
    color: Vec4,     // 16: 64-79
}

impl Sdf2dPrim {
    const CIRCLE: u32 = 0;
    const CAPSULE: u32 = 1;
    const RECT: u32 = 2;
    const ELLIPSE: u32 = 3;
    const UNEVEN_CAPSULE: u32 = 4;
    const ROUNDED_RECT: u32 = 5;
    const CHAMFER_RECT: u32 = 6;
    const RHOMBUS: u32 = 7;
    const TRAPEZOID: u32 = 8;
    const PARALLELOGRAM: u32 = 9;
    const TRIANGLE: u32 = 10;
    const EQU_TRIANGLE: u32 = 11;
    const ISO_TRIANGLE: u32 = 12;
    const PENTAGON: u32 = 13;
    const HEXAGON: u32 = 14;
    const OCTOGON: u32 = 15;
    const PENTAGRAM: u32 = 16;
    const HEXAGRAM: u32 = 17;
    const STAR: u32 = 18;
    const PIE: u32 = 19;
    const CUT_DISK: u32 = 20;
    const ARC: u32 = 21;
    const RING: u32 = 22;
    const HORSESHOE: u32 = 23;
    const VESICA: u32 = 24;
    const MOON: u32 = 25;
    const EGG: u32 = 26;
    const HEART: u32 = 27;
    const ROUNDED_X: u32 = 28;
    const CROSS: u32 = 29;
    const CIRCLE_CROSS: u32 = 30;
    const BLOBBY_CROSS: u32 = 31;
    const PARABOLA: u32 = 32;
    const PARABOLA_SEGMENT: u32 = 33;
    const HYPERBOLA: u32 = 34;
    const QUADRATIC_BEZIER: u32 = 35;
    const QUADRATIC_CIRCLE: u32 = 36;
    const TUNNEL: u32 = 37;
    const STAIRS: u32 = 38;
    const CIRCLE_WAVE: u32 = 39;

    #[allow(clippy::too_many_arguments)]
    fn new(
        a: impl Into<Vec2>,
        b: impl Into<Vec2>,
        c: impl Into<Vec2>,
        radius: f32,
        aux: f32,
        aux_2: u32,
        kind: u32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
            c: c.into(),
            radius,
            aux,
            aux_2,
            kind,
            _pad: Vec2::ZERO,
            m_inv,
            color: color.into(),
        }
    }

    pub fn circle(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            radius,
            0.,
            0,
            Self::CIRCLE,
            m_inv,
            color,
        )
    }

    pub fn capsule(
        origin: impl Into<Vec2>,
        length: f32,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            origin,
            Vec2::new(length, 0.),
            Vec2::ZERO,
            radius,
            0.,
            0,
            Self::CAPSULE,
            m_inv,
            color,
        )
    }

    pub fn rect(
        center: impl Into<Vec2>,
        half_size: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            half_size,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::RECT,
            m_inv,
            color,
        )
    }

    /// `ab` are the ellipse semi-axes.
    pub fn ellipse(
        center: impl Into<Vec2>,
        ab: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            ab,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::ELLIPSE,
            m_inv,
            color,
        )
    }

    /// Capsule with radius `origin_radius` at the origin and `tip_radius` at
    /// `(0, length)`.
    pub fn uneven_capsule(
        center: impl Into<Vec2>,
        length: f32,
        origin_radius: f32,
        tip_radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(length, 0.),
            Vec2::ZERO,
            origin_radius,
            tip_radius,
            0,
            Self::UNEVEN_CAPSULE,
            m_inv,
            color,
        )
    }

    /// Corner radii are ordered like Inigo's `sdRoundedBox`: `xy` for
    /// `p.x > 0`, `zw` for `p.x <= 0`, and within each pair `x` is `p.y > 0`.
    pub fn rounded_rect(
        center: impl Into<Vec2>,
        half_size: impl Into<Vec2>,
        corner_radii: Vec4,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            half_size,
            Vec2::new(corner_radii.z, corner_radii.w),
            corner_radii.x,
            corner_radii.y,
            0,
            Self::ROUNDED_RECT,
            m_inv,
            color,
        )
    }

    /// Corner chamfers use the same quadrant ordering as [`Self::rounded_rect`].
    pub fn chamfer_rect(
        center: impl Into<Vec2>,
        half_size: impl Into<Vec2>,
        chamfers: Vec4,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            half_size,
            Vec2::new(chamfers.z, chamfers.w),
            chamfers.x,
            chamfers.y,
            0,
            Self::CHAMFER_RECT,
            m_inv,
            color,
        )
    }

    /// `half_size` are the half-diagonals of the rhombus.
    pub fn rhombus(
        center: impl Into<Vec2>,
        half_size: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            half_size,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::RHOMBUS,
            m_inv,
            color,
        )
    }

    pub fn trapezoid(
        center: impl Into<Vec2>,
        bottom_radius: f32,
        top_radius: f32,
        half_height: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(half_height, 0.),
            Vec2::ZERO,
            bottom_radius,
            top_radius,
            0,
            Self::TRAPEZOID,
            m_inv,
            color,
        )
    }

    pub fn parallelogram(
        center: impl Into<Vec2>,
        width: f32,
        height: f32,
        skew: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(width, height),
            Vec2::ZERO,
            skew,
            0.,
            0,
            Self::PARALLELOGRAM,
            m_inv,
            color,
        )
    }

    pub fn triangle(
        center: impl Into<Vec2>,
        p0: impl Into<Vec2>,
        p1: impl Into<Vec2>,
        p2: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        let p2 = p2.into();
        Self::new(center, p0, p1, p2.x, p2.y, 0, Self::TRIANGLE, m_inv, color)
    }

    pub fn equ_triangle(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            radius,
            0.,
            0,
            Self::EQU_TRIANGLE,
            m_inv,
            color,
        )
    }

    /// Isosceles triangle with its apex at the origin and its base at
    /// `y = height`, spanning `[-half_base, half_base]`.
    pub fn iso_triangle(
        center: impl Into<Vec2>,
        height: f32,
        half_base: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(half_base, height),
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::ISO_TRIANGLE,
            m_inv,
            color,
        )
    }

    fn regular_polygon(
        center: impl Into<Vec2>,
        radius: f32,
        kind: u32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            radius,
            0.,
            0,
            kind,
            m_inv,
            color,
        )
    }

    pub fn pentagon(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::regular_polygon(center, radius, Self::PENTAGON, m_inv, color)
    }

    pub fn hexagon(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::regular_polygon(center, radius, Self::HEXAGON, m_inv, color)
    }

    pub fn octogon(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::regular_polygon(center, radius, Self::OCTOGON, m_inv, color)
    }

    pub fn pentagram(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::regular_polygon(center, radius, Self::PENTAGRAM, m_inv, color)
    }

    pub fn hexagram(
        center: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::regular_polygon(center, radius, Self::HEXAGRAM, m_inv, color)
    }

    /// `inner_radius_factor` is Inigo's `m` and must be in `(0, point_count]`;
    /// `point_count` is `n`.
    pub fn star(
        center: impl Into<Vec2>,
        outer_radius: f32,
        inner_radius_factor: f32,
        point_count: u32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            outer_radius,
            inner_radius_factor,
            point_count,
            Self::STAR,
            m_inv,
            color,
        )
    }

    /// `open_angle` is the sine/cosine pair of the pie aperture.
    pub fn pie(
        center: impl Into<Vec2>,
        open_angle: impl Into<Vec2>,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            open_angle,
            Vec2::ZERO,
            radius,
            0.,
            0,
            Self::PIE,
            m_inv,
            color,
        )
    }

    pub fn cut_disk(
        center: impl Into<Vec2>,
        radius: f32,
        cut_height: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(cut_height, 0.),
            Vec2::ZERO,
            radius,
            0.,
            0,
            Self::CUT_DISK,
            m_inv,
            color,
        )
    }

    /// `open_angle` is the sine/cosine pair of the arc aperture.
    pub fn arc(
        center: impl Into<Vec2>,
        open_angle: impl Into<Vec2>,
        radius: f32,
        thickness: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            open_angle,
            Vec2::ZERO,
            radius,
            thickness,
            0,
            Self::ARC,
            m_inv,
            color,
        )
    }

    pub fn ring(
        center: impl Into<Vec2>,
        normal: impl Into<Vec2>,
        radius: f32,
        thickness: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            normal,
            Vec2::ZERO,
            radius,
            thickness,
            0,
            Self::RING,
            m_inv,
            color,
        )
    }

    /// `open_angle` is the sine/cosine pair of the aperture and `thickness`
    /// is the per-axis leg width.
    pub fn horseshoe(
        center: impl Into<Vec2>,
        open_angle: impl Into<Vec2>,
        radius: f32,
        thickness: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            open_angle,
            thickness,
            radius,
            0.,
            0,
            Self::HORSESHOE,
            m_inv,
            color,
        )
    }

    pub fn vesica(
        center: impl Into<Vec2>,
        width: f32,
        height: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(width, height),
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::VESICA,
            m_inv,
            color,
        )
    }

    pub fn moon(
        center: impl Into<Vec2>,
        center_distance: f32,
        outer_radius: f32,
        inner_radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(center_distance, 0.),
            Vec2::ZERO,
            outer_radius,
            inner_radius,
            0,
            Self::MOON,
            m_inv,
            color,
        )
    }

    /// `top_radius` is the radius at `(0, half_height)`, `bottom_radius` at
    /// the origin. `bulge` is Inigo's `bu`.
    pub fn egg(
        center: impl Into<Vec2>,
        half_height: f32,
        top_radius: f32,
        bottom_radius: f32,
        bulge: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(half_height, bulge),
            Vec2::ZERO,
            bottom_radius,
            top_radius,
            0,
            Self::EGG,
            m_inv,
            color,
        )
    }

    pub fn heart(center: impl Into<Vec2>, m_inv: [f32; 4], color: impl Into<Vec4>) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::HEART,
            m_inv,
            color,
        )
    }

    pub fn rounded_x(
        center: impl Into<Vec2>,
        thickness: f32,
        corner_radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            corner_radius,
            thickness,
            0,
            Self::ROUNDED_X,
            m_inv,
            color,
        )
    }

    pub fn cross(
        center: impl Into<Vec2>,
        arm_half_extent: impl Into<Vec2>,
        corner_radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            arm_half_extent,
            Vec2::ZERO,
            corner_radius,
            0.,
            0,
            Self::CROSS,
            m_inv,
            color,
        )
    }

    pub fn circle_cross(
        center: impl Into<Vec2>,
        arm_extent: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            arm_extent,
            0.,
            0,
            Self::CIRCLE_CROSS,
            m_inv,
            color,
        )
    }

    pub fn blobby_cross(
        center: impl Into<Vec2>,
        half_extent: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            half_extent,
            0.,
            0,
            Self::BLOBBY_CROSS,
            m_inv,
            color,
        )
    }

    pub fn parabola(
        center: impl Into<Vec2>,
        curvature: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            curvature,
            0.,
            0,
            Self::PARABOLA,
            m_inv,
            color,
        )
    }

    pub fn parabola_segment(
        center: impl Into<Vec2>,
        half_width: f32,
        height: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(half_width, height),
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::PARABOLA_SEGMENT,
            m_inv,
            color,
        )
    }

    pub fn hyperbola(
        center: impl Into<Vec2>,
        curvature: f32,
        half_length: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::new(curvature, half_length),
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::HYPERBOLA,
            m_inv,
            color,
        )
    }

    pub fn quadratic_bezier(
        center: impl Into<Vec2>,
        point_a: impl Into<Vec2>,
        point_b: impl Into<Vec2>,
        point_c: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        let point_c = point_c.into();
        Self::new(
            center,
            point_a,
            point_b,
            point_c.x,
            point_c.y,
            0,
            Self::QUADRATIC_BEZIER,
            m_inv,
            color,
        )
    }

    pub fn quadratic_circle(
        center: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::QUADRATIC_CIRCLE,
            m_inv,
            color,
        )
    }

    pub fn tunnel(
        center: impl Into<Vec2>,
        half_extents: impl Into<Vec2>,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            half_extents,
            Vec2::ZERO,
            0.,
            0.,
            0,
            Self::TUNNEL,
            m_inv,
            color,
        )
    }

    pub fn stairs(
        center: impl Into<Vec2>,
        step_size: impl Into<Vec2>,
        count: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            step_size,
            Vec2::ZERO,
            count,
            0.,
            0,
            Self::STAIRS,
            m_inv,
            color,
        )
    }

    pub fn circle_wave(
        center: impl Into<Vec2>,
        wave_angle: f32,
        radius: f32,
        m_inv: [f32; 4],
        color: impl Into<Vec4>,
    ) -> Self {
        Self::new(
            center,
            Vec2::ZERO,
            Vec2::ZERO,
            radius,
            wave_angle,
            0,
            Self::CIRCLE_WAVE,
            m_inv,
            color,
        )
    }
}
