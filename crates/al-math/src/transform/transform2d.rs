use crate::{
    matrix::{Mat2, Mat3, Mat4},
    vec::{Vec2, Vec4},
};

#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform2D {
    pub translation: Vec2,
    pub linear: Mat2,
}
super::impl_transform!(Transform2D, Vec2, Mat2);

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        translation: Vec2::ZERO,
        linear: Mat2::IDENTITY,
    };

    #[inline]
    pub fn new(translation: Vec2, rotation: f32, scale: Vec2) -> Self {
        let (s, c) = rotation.sin_cos();
        Self {
            translation,
            linear: Mat2 {
                cols: [
                    Vec2 {
                        x: c * scale.x,
                        y: s * scale.x,
                    },
                    Vec2 {
                        x: -s * scale.y,
                        y: c * scale.y,
                    },
                ],
            },
        }
    }

    #[inline]
    pub fn from_rotation(rotation: f32) -> Self {
        Self::new(Vec2::ZERO, rotation, Vec2::ONE)
    }

    #[inline]
    pub fn from_translation(translation: Vec2) -> Self {
        Self {
            translation,
            linear: Self::IDENTITY.linear,
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec2) -> Self {
        Self {
            translation: Vec2::ZERO,
            linear: Mat2 {
                cols: [Vec2 { x: scale.x, y: 0.0 }, Vec2 { x: 0.0, y: scale.y }],
            },
        }
    }

    #[inline]
    pub fn with_rotation_mat(mut self, rotation: Mat2) -> Self {
        let (_, _, scale) = self.to_components();
        self.linear = rotation * Mat2::from_scale(scale);
        self
    }

    #[inline]
    pub fn with_rotation(mut self, radians: f32) -> Self {
        let (_, _, scale) = self.to_components();
        self = Self::new(self.translation, radians, scale);
        self
    }

    #[inline]
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        let (_, rot, _) = self.to_components();
        self = Self::new(self.translation, rot, scale);
        self
    }

    /// Returns (translation, rotation, scale).
    #[inline]
    pub fn to_components(self) -> (Vec2, f32, Vec2) {
        let c0 = self.linear.cols[0];
        let c1 = self.linear.cols[1];

        let sx = c0.length();
        let sy = c1.length();
        let rotation = c0.y.atan2(c0.x);

        let det = self.linear.determinant();
        let sy = if det < 0.0 { -sy } else { sy };

        (self.translation, rotation, Vec2::new(sx, sy))
    }

    #[inline]
    pub fn to_mat3(self) -> Mat3 {
        Mat3::new(
            self.linear.cols[0].extend(0.0),
            self.linear.cols[1].extend(0.0),
            self.translation.extend(1.0),
        )
    }

    #[inline]
    pub fn from_mat3(m: Mat3) -> Option<Self> {
        if m.cols[0].z != 0.0 || m.cols[1].z != 0.0 || m.cols[2].z != 1.0 {
            return None;
        }

        Some(Self {
            translation: Vec2::new(m.cols[2].x, m.cols[2].y),
            linear: Mat2::new(
                Vec2::new(m.cols[0].x, m.cols[0].y),
                Vec2::new(m.cols[1].x, m.cols[1].y),
            ),
        })
    }

    #[inline]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::new(
            self.linear.cols[0].extend(0.0).extend(0.0),
            self.linear.cols[1].extend(0.0).extend(0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(self.translation.x, self.translation.y, 0.0, 1.0),
        )
    }
}
