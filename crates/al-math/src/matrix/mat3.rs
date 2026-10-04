use crate::{
    matrix::Mat4,
    vec::{Vec2, Vec3, Vec3A, Vec4},
};

#[must_use]
#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mat3 {
    pub cols: [Vec3; 3],
}

super::define_mat_impls!(Mat3, Vec3, 3, c0, c1, c2; [
    Vec3 { x: 1., y: 0., z: 0. },
    Vec3 { x: 0., y: 1., z: 0. },
    Vec3 { x: 0., y: 0., z: 1. },
]);
mat3_helpers!(Mat3, Vec3);
super::define_dense_mat_refs!(Mat3, 3);
#[cfg(feature = "gpu")]
encase::impl_matrix!(3, 3, Mat3, f32; using AsRef AsMut From);

#[must_use]
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mat3A {
    pub cols: [Vec3A; 3],
}

super::define_mat_impls!(Mat3A, Vec3A, 3, c0, c1, c2; [
    Vec3A { x: 1., y: 0., z: 0., _pad: 0. },
    Vec3A { x: 0., y: 1., z: 0., _pad: 0. },
    Vec3A { x: 0., y: 0., z: 1., _pad: 0. },
]);
mat3_helpers!(Mat3A, Vec3A);

impl From<Mat3> for Mat3A {
    #[inline(always)]
    fn from(m: Mat3) -> Self {
        Self::from_cols(m.cols.map(Vec3A::from))
    }
}
impl From<Mat3A> for Mat3 {
    #[inline(always)]
    fn from(m: Mat3A) -> Self {
        Self::from_cols(m.cols.map(Vec3::from))
    }
}

macro_rules! mat3_helpers {
    ($mat3:ident, $vec3:ident) => {
        impl $mat3 {
            #[inline]
            pub fn from_rotation_x(radians: f32) -> Self {
                let (s, c) = radians.sin_cos();
                Self::new(
                    $vec3::new(1.0, 0.0, 0.0),
                    $vec3::new(0.0, c, s),
                    $vec3::new(0.0, -s, c),
                )
            }

            #[inline]
            pub fn from_rotation_y(radians: f32) -> Self {
                let (s, c) = radians.sin_cos();
                Self::new(
                    $vec3::new(c, 0.0, -s),
                    $vec3::new(0.0, 1.0, 0.0),
                    $vec3::new(s, 0.0, c),
                )
            }

            #[inline]
            pub fn from_rotation_z(radians: f32) -> Self {
                let (s, c) = radians.sin_cos();
                Self::new(
                    $vec3::new(c, s, 0.0),
                    $vec3::new(-s, c, 0.0),
                    $vec3::new(0.0, 0.0, 1.0),
                )
            }

            #[inline]
            pub fn from_axis_angle(axis: $vec3, radians: f32) -> Self {
                let (s, c) = radians.sin_cos();
                let t = 1.0 - c;
                let a = axis.normalize();
                let (x, y, z) = (a.x, a.y, a.z);
                Self::new(
                    $vec3::new(t * x * x + c, t * x * y + s * z, t * x * z - s * y),
                    $vec3::new(t * x * y - s * z, t * y * y + c, t * y * z + s * x),
                    $vec3::new(t * x * z + s * y, t * y * z - s * x, t * z * z + c),
                )
            }

            #[inline]
            pub fn from_scale(scale: $vec3) -> Self {
                Self::new(
                    $vec3::new(scale.x, 0.0, 0.0),
                    $vec3::new(0.0, scale.y, 0.0),
                    $vec3::new(0.0, 0.0, scale.z),
                )
            }

            #[inline]
            pub fn from_mat4(m: super::Mat4) -> Self {
                let c = m.cols;
                Self::new(
                    c[0].truncate().into(),
                    c[1].truncate().into(),
                    c[2].truncate().into(),
                )
            }

            /// Embed in the upper-left of a 4×4 with 1 at [3][3].
            #[inline]
            pub fn extend_to_mat4(self) -> Mat4 {
                Mat4::new(
                    self.cols[0].extend(0.0),
                    self.cols[1].extend(0.0),
                    self.cols[2].extend(0.0),
                    Vec4::new(0.0, 0.0, 0.0, 1.0),
                )
            }

            /// Upper-left 3×3 determinant
            #[must_use]
            #[inline]
            pub fn determinant(&self) -> f32 {
                self.cols[0].dot(self.cols[1].cross(self.cols[2]))
            }

            #[must_use]
            pub fn try_inverse(&self) -> Option<Self> {
                let det = self.determinant();
                if det.abs() < f32::EPSILON {
                    return None;
                }
                let inv = 1.0 / det;
                let c = &self.cols;
                Some(Self::new(
                    c[1].cross(c[2]) * inv,
                    c[2].cross(c[0]) * inv,
                    c[0].cross(c[1]) * inv,
                ))
            }

            #[inline]
            pub fn from_diagonal(diag: $vec3) -> Self {
                let mut m = Self::ZERO;
                m.cols[0].x = diag.x;
                m.cols[1].y = diag.y;
                m.cols[2].z = diag.z;
                m
            }

            #[inline]
            pub fn from_angle2d(radians: f32) -> Self {
                let (s, c) = radians.sin_cos();
                Self::new(
                    $vec3::new(c, s, 0.0),
                    $vec3::new(-s, c, 0.0),
                    $vec3::new(0.0, 0.0, 1.0),
                )
            }

            #[inline]
            pub fn from_translation2d(t: Vec2) -> Self {
                Self::new(
                    $vec3::new(1.0, 0.0, 0.0),
                    $vec3::new(0.0, 1.0, 0.0),
                    $vec3::new(t.x, t.y, 1.0),
                )
            }

            #[inline]
            pub fn from_scale2d(s: Vec2) -> Self {
                Self::new(
                    $vec3::new(s.x, 0.0, 0.0),
                    $vec3::new(0.0, s.y, 0.0),
                    $vec3::new(0.0, 0.0, 1.0),
                )
            }

            #[inline]
            pub fn transform_point2d(self, p: Vec2) -> Vec2 {
                let c = &self.cols;
                Vec2::new(
                    c[0].x * p.x + c[1].x * p.y + c[2].x,
                    c[0].y * p.x + c[1].y * p.y + c[2].y,
                )
            }

            #[inline]
            pub fn transform_vector2d(self, v: Vec2) -> Vec2 {
                let c = &self.cols;
                Vec2::new(c[0].x * v.x + c[1].x * v.y, c[0].y * v.x + c[1].y * v.y)
            }
        }
    };
}

use mat3_helpers;
