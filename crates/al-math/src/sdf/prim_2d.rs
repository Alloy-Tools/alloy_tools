use crate::vec::{Vec2, Vec4};

pub const SDF_PRIM_SIZE: usize = std::mem::size_of::<Sdf2dPrim>();

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
    m_inv: [f32; 4], // 16: 32-47
    color: Vec4,     // 16: 48-63
}

impl Sdf2dPrim {
    //REVIEW: Use variant "id" directly
    const CIRCLE: u32 = 0;
    const CAPSULE: u32 = 1;
    const RECT: u32 = 2;

    fn new(
        a: impl Into<Vec2>,
        b: impl Into<Vec2>,
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
            radius,
            aux,
            aux_2,
            kind,
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
        Self::new(center, half_size, 0., 0., 0, Self::RECT, m_inv, color)
    }
}
