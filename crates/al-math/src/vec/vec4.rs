use super::Vec3;

#[must_use]
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

super::define_vec_impls!(Vec4, 4, x, y, z, w);
#[cfg(feature = "gpu")]
encase::impl_vector!(4, Vec4, f32; using AsRef AsMut From);

impl Vec4 {
    pub const UP: Self = Self {
        x: 0.,
        y: 1.,
        z: 0.,
        w: 0.,
    };
    pub const RIGHT: Self = Self {
        x: 1.,
        y: 0.,
        z: 0.,
        w: 0.,
    };
    pub const FORWARD: Self = Self {
        x: 0.,
        y: 0.,
        z: 1.,
        w: 0.,
    };

    #[inline]
    pub fn from_vec3(v: Vec3, w: f32) -> Self {
        Self::new(v.x, v.y, v.z, w)
    }

    #[inline]
    pub fn project_to_vec3(self) -> Vec3 {
        if self.w == 0.0 || self.w.is_nan() {
            self.truncate()
        } else {
            let rec = 1.0 / self.w;
            Vec3::new(self.x * rec, self.y * rec, self.z * rec)
        }
    }

    #[inline]
    pub fn truncate(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    #[inline]
    pub fn r(self) -> f32 {
        self.x
    }
    #[inline]
    pub fn g(self) -> f32 {
        self.y
    }
    #[inline]
    pub fn b(self) -> f32 {
        self.z
    }
    #[inline]
    pub fn a(self) -> f32 {
        self.w
    }
}
