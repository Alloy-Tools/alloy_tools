use super::{Vec2, Vec4};

#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

super::define_vec_impls!(Vec3, 3, x, y, z);
vec3_helpers!(Vec3);
#[cfg(feature = "gpu")]
encase::impl_vector!(3, Vec3, f32; using AsRef AsMut From);

#[must_use]
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Vec3A {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    _pad: f32,
}

super::define_vec_impls!(Vec3A, 3, x, y, z; _pad);
vec3_helpers!(Vec3A, _pad);

impl From<Vec3> for Vec3A {
    #[inline(always)]
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3A> for Vec3 {
    #[inline(always)]
    fn from(v: Vec3A) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

macro_rules! vec3_helpers {
    ($vec:ident $(, $pad:ident)?) => {
        impl $vec {
            pub const UP: Self = Self {
                x: 0.,
                y: 1.,
                z: 0.,
                $($pad: 0.0,)?
            };
            pub const RIGHT: Self = Self {
                x: 1.,
                y: 0.,
                z: 0.,
                $($pad: 0.0,)?
            };
            pub const FORWARD: Self = Self {
                x: 0.,
                y: 0.,
                z: 1.,
                $($pad: 0.0,)?
            };

            #[inline]
            pub fn cross(self, other: Self) -> Self {
                Self::new(
                    self.y * other.z - self.z * other.y,
                    self.z * other.x - self.x * other.z,
                    self.x * other.y - self.y * other.x,
                )
            }

            #[must_use]
            #[inline]
            pub fn angle_between(self, other: Self) -> f32 {
                (self.dot(other) / (self.length() * other.length()))
                    .clamp(-1.0, 1.0)
                    .acos()
            }

            #[inline]
            pub fn reflect(self, normal: Self) -> Self {
                self - normal * (2.0 * self.dot(normal))
            }

            #[inline]
            pub fn project_onto(self, onto: Self) -> Self {
                onto * (self.dot(onto) / onto.length_squared())
            }

            #[inline]
            pub fn reject_from(self, from: Self) -> Self {
                self - self.project_onto(from)
            }

            #[inline]
            pub fn truncate(self) -> Vec2 {
                Vec2::new(self.x, self.y)
            }

            #[inline]
            pub fn extend(self, w: f32) -> Vec4 {
                Vec4::new(self.x, self.y, self.z, w)
            }
        }
    };
}

use vec3_helpers;
