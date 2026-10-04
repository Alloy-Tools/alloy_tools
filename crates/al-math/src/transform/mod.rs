mod transform2d;
mod transform3d;

pub use transform2d::Transform2D;
pub use transform3d::Transform3D;

macro_rules! impl_transform {
    ($transform:ident, $vec:ident, $mat:ident) => {
        impl $transform {
            #[inline]
            pub fn with_translation(mut self, t: $vec) -> Self {
                self.translation = t;
                self
            }

            #[inline]
            pub fn with_linear(mut self, l: $mat) -> Self {
                self.linear = l;
                self
            }

            #[inline]
            pub fn compose(&self, child: &Self) -> Self {
                $transform {
                    translation: self.translation + self.linear * child.translation,
                    linear: self.linear * child.linear,
                }
            }

            #[inline]
            pub fn precompose(&self, other: &Self) -> Self {
                other.compose(self)
            }

            /// Alias for `compose`.
            #[inline]
            pub fn then(&self, child: &Self) -> Self {
                self.compose(child)
            }

            /// Transform point from local space to world space.
            #[inline]
            pub fn transform_point(&self, p: $vec) -> $vec {
                self.translation + self.linear * p
            }

            /// Transform vector from local space to world space.
            #[inline]
            pub fn transform_vector(&self, v: $vec) -> $vec {
                self.linear * v
            }

            #[inline]
            pub fn try_inverse(&self) -> Option<Self> {
                let inv_linear = self.linear.try_inverse()?;
                Some(Self {
                    translation: -(inv_linear * self.translation),
                    linear: inv_linear,
                })
            }

            /// Transform point from world space to local space.
            #[inline]
            pub fn inverse_transform_point(&self, p: $vec) -> Option<$vec> {
                self.try_inverse().map(|inv| inv.transform_point(p))
            }

            /// Transform vector from world space to local space.
            #[inline]
            pub fn inverse_transform_vector(&self, v: $vec) -> Option<$vec> {
                self.linear.try_inverse().map(|inv| inv * v)
            }

            #[inline]
            pub fn approx_eq(&self, other: &Self, eps: f32) -> bool {
                self.translation.approx_eq(other.translation, eps)
                    && self.linear.approx_eq(&other.linear, eps)
            }

            #[inline]
            pub fn is_finite(&self) -> bool {
                self.translation.is_finite() && self.linear.is_finite()
            }
        }
    };
}

use impl_transform;
