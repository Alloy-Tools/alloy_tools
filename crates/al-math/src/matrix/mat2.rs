use crate::vec::Vec2;

#[must_use]
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Mat2 {
    pub cols: [Vec2; 2],
}

super::define_mat_impls!(Mat2, Vec2, 2, c0, c1; [
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 1.0),
]);
super::define_dense_mat_refs!(Mat2, 2);
#[cfg(feature = "gpu")]
encase::impl_matrix!(2, 2, Mat2, f32; using AsRef AsMut From);

impl Mat2 {
    #[inline]
    pub fn from_angle(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self::new(Vec2::new(cos, sin), Vec2::new(-sin, cos))
    }

    #[inline]
    pub fn from_scale(scale: Vec2) -> Self {
        Self::new(Vec2::new(scale.x, 0.0), Vec2::new(0.0, scale.y))
    }

    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.cols[0].x * self.cols[1].y - self.cols[1].x * self.cols[0].y
    }

    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < f32::EPSILON { return None; }
        let inv = 1.0 / det;
        Some(Self::new(
            Vec2::new( self.cols[1].y * inv, -self.cols[0].y * inv),
            Vec2::new(-self.cols[1].x * inv,  self.cols[0].x * inv),
        ))
    }
}
