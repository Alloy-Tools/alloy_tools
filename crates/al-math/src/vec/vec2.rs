use super::Vec3;

#[must_use]
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable,))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

super::define_vec_impls!(Vec2, 2, x, y);
#[cfg(feature = "gpu")]
encase::impl_vector!(2, Vec2, f32; using AsRef AsMut From);

impl Vec2 {
    pub const UP: Self = Self { x: 0., y: 1. };
    pub const RIGHT: Self = Self { x: 1., y: 0. };

    #[inline]
    pub fn rotate(self, radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }
    #[inline]
    pub fn perpendicular(self) -> Self {
        Self::new(-self.y, self.x)
    }
    #[inline]
    pub fn perpendicular_clockwise(self) -> Self {
        Self::new(self.y, -self.x)
    }
    #[inline]
    pub fn extend(self, z: f32) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }
    #[must_use]
    #[inline]
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    #[must_use]
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        (self.dot(other) / (self.length() * other.length()))
            .clamp(-1.0, 1.0)
            .acos()
    }
}
