use crate::{
    matrix::{Mat3, Mat4},
    vec::Vec3,
};

#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform3D {
    pub translation: Vec3,
    pub linear: Mat3,
}
super::impl_transform!(Transform3D, Vec3, Mat3);

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform3D {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        linear: Mat3::IDENTITY,
    };

    #[inline]
    pub fn new(translation: Vec3, rotation: Mat3, scale: Vec3) -> Self {
        Self {
            translation,
            linear: rotation * Mat3::from_scale(scale),
        }
    }

    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            linear: Self::IDENTITY.linear,
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            translation: Vec3::ZERO,
            linear: Mat3::from_scale(scale),
        }
    }

    /// Returns (translation, rotation, scale).
    #[inline]
    pub fn to_components(self) -> (Vec3, Mat3, Vec3) {
        let c0 = self.linear.cols[0];
        let c1 = self.linear.cols[1];
        let c2 = self.linear.cols[2];

        let sx = c0.length();
        let sy = c1.length();
        let sz = c2.length();

        // Gram-Schmidt orthonormalize
        let r0 = c0 / sx;
        let r1_raw = c1 - r0 * c1.dot(r0);
        let r1 = r1_raw.normalize();
        let r2_raw = c2 - r0 * c2.dot(r0) - r1 * c2.dot(r1);
        let r2 = r2_raw.normalize();

        // preserve handedness
        let det = r0.dot(r1.cross(r2));
        let r2 = if det < 0.0 { -r2 } else { r2 };

        let rotation = Mat3::new(r0, r1, r2);
        (self.translation, rotation, Vec3::new(sx, sy, sz))
    }

    /// Replace the linear part with `rotation * scale`, keeping the existing scale.
    #[inline]
    pub fn with_rotation_mat(mut self, rotation: Mat3) -> Self {
        let (_, _, scale) = self.to_components();
        self.linear = rotation * Mat3::from_scale(scale);
        self
    }

    /// Replace the rotation by axis/angle, keeping the existing scale.
    #[inline]
    pub fn with_axis_angle(mut self, axis: Vec3, radians: f32) -> Self {
        let (_, _, scale) = self.to_components();
        self.linear = Mat3::from_axis_angle(axis, radians) * Mat3::from_scale(scale);
        self
    }

    #[inline]
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        let (_, rotation, _) = self.to_components();
        self.linear = rotation * Mat3::from_scale(scale);
        self
    }

    #[inline]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::new(
            self.linear.cols[0].extend(0.0),
            self.linear.cols[1].extend(0.0),
            self.linear.cols[2].extend(0.0),
            self.translation.extend(1.0),
        )
    }

    #[inline]
    pub fn from_mat4(m: Mat4) -> Option<Self> {
        if m.cols[0].w != 0.0 || m.cols[1].w != 0.0 || m.cols[2].w != 0.0 || m.cols[3].w != 1.0 {
            return None;
        }

        Some(Self {
            translation: m.cols[3].truncate(),
            linear: Mat3::new(
                m.cols[0].truncate(),
                m.cols[1].truncate(),
                m.cols[2].truncate(),
            ),
        })
    }
}
