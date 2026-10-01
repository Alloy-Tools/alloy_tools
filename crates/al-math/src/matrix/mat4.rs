use crate::{
    matrix::Mat3,
    vec::{Vec3, Vec4},
};

#[must_use]
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "gpu", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Mat4 {
    pub cols: [Vec4; 4],
}

super::define_mat_impls!(Mat4, Vec4, 4, c0, c1, c2, c3; [
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, 0.0, 0.0, 1.0),
]);
super::define_dense_mat_refs!(Mat4, 4);
#[cfg(feature = "gpu")]
encase::impl_matrix!(4, 4, Mat4, f32; using AsRef AsMut From);

impl Mat4 {
    #[inline]
    pub fn from_translation(t: Vec3) -> Self {
        let mut m = Self::identity();
        m.cols[3] = Vec4::from_vec3(t, 1.0);
        m
    }

    #[inline]
    pub fn from_scale(s: Vec3) -> Self {
        let mut m = Self::identity();
        m.cols[0].x = s.x;
        m.cols[1].y = s.y;
        m.cols[2].z = s.z;
        m
    }

    #[inline]
    pub fn from_rotation_x(r: f32) -> Self {
        let (s, c) = r.sin_cos();
        Self::new(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, c, s, 0.0),
            Vec4::new(0.0, -s, c, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    #[inline]
    pub fn from_rotation_y(r: f32) -> Self {
        let (s, c) = r.sin_cos();
        Self::new(
            Vec4::new(c, 0.0, -s, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(s, 0.0, c, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    #[inline]
    pub fn from_rotation_z(r: f32) -> Self {
        let (s, c) = r.sin_cos();
        Self::new(
            Vec4::new(c, s, 0.0, 0.0),
            Vec4::new(-s, c, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    /// Rotation from a normalized axis and angle.
    pub fn from_axis_angle(axis: Vec3, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        let t = 1.0 - c;
        let a = axis.normalize();
        let (x, y, z) = (a.x, a.y, a.z);
        Self::new(
            Vec4::new(t * x * x + c, t * x * y + s * z, t * x * z - s * y, 0.0),
            Vec4::new(t * x * y - s * z, t * y * y + c, t * y * z + s * x, 0.0),
            Vec4::new(t * x * z + s * y, t * y * z - s * x, t * z * z + c, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    /// Right-handed view matrix for wgpu's coordinate system.
    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize(); // forward
        let s = f.cross(up).normalize(); // right
        let u = s.cross(f); // corrected up
        Self::new(
            Vec4::new(s.x, u.x, -f.x, 0.0),
            Vec4::new(s.y, u.y, -f.y, 0.0),
            Vec4::new(s.z, u.z, -f.z, 0.0),
            Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    /// Right-handed perspective projection for the [0, 1] depth range
    pub fn perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y * 0.5).tan();
        let nf = 1.0 / (near - far);
        Self::new(
            Vec4::new(f / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, far * nf, -1.0),
            Vec4::new(0.0, 0.0, near * far * nf, 0.0),
        )
    }

    pub fn orthographic_rh(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let rcp_w = 1.0 / (right - left);
        let rcp_h = 1.0 / (top - bottom);
        let r = 1.0 / (near - far);
        Self::new(
            Vec4::new(rcp_w + rcp_w, 0.0, 0.0, 0.0),
            Vec4::new(0.0, rcp_h + rcp_h, 0.0, 0.0),
            Vec4::new(0.0, 0.0, r, 0.0),
            Vec4::new(
                -(left + right) * rcp_w,
                -(top + bottom) * rcp_h,
                near * r,
                1.0,
            ),
        )
    }

    #[must_use]
    #[inline]
    pub fn determinant(&self) -> f32 {
        let c = &self.cols;
        let b00 = c[0].x * c[1].y - c[0].y * c[1].x;
        let b01 = c[0].x * c[1].z - c[0].z * c[1].x;
        let b02 = c[0].x * c[1].w - c[0].w * c[1].x;
        let b03 = c[0].y * c[1].z - c[0].z * c[1].y;
        let b04 = c[0].y * c[1].w - c[0].w * c[1].y;
        let b05 = c[0].z * c[1].w - c[0].w * c[1].z;
        let b06 = c[2].x * c[3].y - c[2].y * c[3].x;
        let b07 = c[2].x * c[3].z - c[2].z * c[3].x;
        let b08 = c[2].x * c[3].w - c[2].w * c[3].x;
        let b09 = c[2].y * c[3].z - c[2].z * c[3].y;
        let b10 = c[2].y * c[3].w - c[2].w * c[3].y;
        let b11 = c[2].z * c[3].w - c[2].w * c[3].z;
        b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06
    }

    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let c = &self.cols;
        // Row-major elements
        let (m00, m01, m02, m03) = (c[0].x, c[1].x, c[2].x, c[3].x);
        let (m10, m11, m12, m13) = (c[0].y, c[1].y, c[2].y, c[3].y);
        let (m20, m21, m22, m23) = (c[0].z, c[1].z, c[2].z, c[3].z);
        let (m30, m31, m32, m33) = (c[0].w, c[1].w, c[2].w, c[3].w);

        // 2x2 determinants
        let s0 = m00 * m11 - m10 * m01;
        let s1 = m00 * m12 - m10 * m02;
        let s2 = m00 * m13 - m10 * m03;
        let s3 = m01 * m12 - m11 * m02;
        let s4 = m01 * m13 - m11 * m03;
        let s5 = m02 * m13 - m12 * m03;
        let s6 = m20 * m31 - m30 * m21;
        let s7 = m20 * m32 - m30 * m22;
        let s8 = m20 * m33 - m30 * m23;
        let s9 = m21 * m32 - m31 * m22;
        let s10 = m21 * m33 - m31 * m23;
        let s11 = m22 * m33 - m32 * m23;

        // 3x3 cofactors
        let c00 = m11 * s11 - m12 * s10 + m13 * s9;
        let c01 = m10 * s11 - m12 * s8 + m13 * s7;
        let c02 = m10 * s10 - m11 * s8 + m13 * s6;
        let c03 = m10 * s9 - m11 * s7 + m12 * s6;
        let c04 = m01 * s11 - m02 * s10 + m03 * s9;
        let c05 = m00 * s11 - m02 * s8 + m03 * s7;
        let c06 = m00 * s10 - m01 * s8 + m03 * s6;
        let c07 = m00 * s9 - m01 * s7 + m02 * s6;
        let c08 = m31 * s5 - m32 * s4 + m33 * s3;
        let c09 = m30 * s5 - m32 * s2 + m33 * s1;
        let c10 = m30 * s4 - m31 * s2 + m33 * s0;
        let c11 = m30 * s3 - m31 * s1 + m32 * s0;
        let c12 = m21 * s5 - m22 * s4 + m23 * s3;
        let c13 = m20 * s5 - m22 * s2 + m23 * s1;
        let c14 = m20 * s4 - m21 * s2 + m23 * s0;
        let c15 = m20 * s3 - m21 * s1 + m22 * s0;

        let det = m00 * c00 - m01 * c01 + m02 * c02 - m03 * c03;
        if det.abs() < f32::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;

        // Transpose of the cofactor matrix, scaled by 1/det.
        Some(Self::new(
            Vec4::new(c00, -c01, c02, -c03) * inv_det,
            Vec4::new(-c04, c05, -c06, c07) * inv_det,
            Vec4::new(c08, -c09, c10, -c11) * inv_det,
            Vec4::new(-c12, c13, -c14, c15) * inv_det,
        ))
    }

    /// Drop the last row and column.
    #[inline]
    pub fn to_mat3(self) -> Mat3 {
        Mat3::from_mat4(self)
    }

    /// Point transform.
    #[inline]
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        (self * Vec4::from_vec3(p, 1.0)).project_to_vec3()
    }

    /// Direction transform.
    #[inline]
    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        (self * Vec4::from_vec3(v, 0.0)).truncate()
    }
}
