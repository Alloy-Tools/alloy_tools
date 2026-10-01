mod mat2;
mod mat3;
mod mat4;

pub use mat2::Mat2;
pub use mat3::{Mat3, Mat3A};
pub use mat4::Mat4;

macro_rules! define_mat_impls {
    ($mat_name:ident, $vec_name:ident, $dim:expr, $($col:ident),+; [$($ident_col:expr,)+]) => {
        impl $mat_name {
            pub const ZERO: Self = Self { cols: [$vec_name::ZERO; $dim] };

            #[inline]
            pub fn new($($col: $vec_name),+) -> Self {
                Self { cols: [$($col),+] }
            }

            /// Creates a matrix from an array of column vectors.
            #[inline]
            pub fn from_cols(cols: [$vec_name; $dim]) -> Self {
                Self { cols }
            }

            /// Returns the identity matrix.
            #[inline]
            pub fn identity() -> Self {
                Self::from_cols([$($ident_col),+])
            }

            #[inline]
            pub fn col(&self, i: usize) -> $vec_name {
                self.cols[i]
            }

            #[inline]
            pub fn col_mut(&mut self, i: usize) -> &mut $vec_name {
                &mut self.cols[i]
            }

            #[inline]
            pub fn row(&self, i: usize) -> $vec_name {
                let mut arr = [0.0f32; $dim];
                for j in 0..$dim {
                    arr[j] = self.cols[j].to_array()[i];
                }
                $vec_name::from(arr)
            }

            /// Swaps rows and columns
            pub fn transpose(&self) -> Self {
                let cols = self.cols;
                Self::from_cols(std::array::from_fn(|i| {
                    let mut arr = [0.0f32; $dim];
                    for j in 0..$dim {
                        arr[j] = cols[j].to_array()[i]
                    }
                    $vec_name::from(arr)
                }))
            }

            #[must_use]
            #[inline]
            pub fn trace(&self) -> f32 {
                let mut s = 0.0;
                for i in 0..$dim {
                    s += self.cols[i].to_array()[i];
                }
                s
            }

            #[must_use]
            #[inline]
            pub fn is_finite(&self) -> bool {
                self.cols.iter().all(|c| c.to_array().iter().all(|f| f32::is_finite(*f)))
            }

            #[must_use]
            #[inline]
            pub fn to_cols_array(&self) -> [f32; { $dim * $dim }] {
                let mut out = [0.0f32; { $dim * $dim }];
                for i in 0..$dim {
                    out[i * $dim..(i + 1) * $dim].copy_from_slice(&self.cols[i].to_array());
                }
                out
            }

            #[must_use]
            #[inline]
            pub fn from_cols_array(arr: &[f32; { $dim * $dim }]) -> Self {
                Self::from_cols(std::array::from_fn(|i| {
                    let mut v = [0.0f32; $dim];
                    v.copy_from_slice(&arr[i * $dim..(i + 1) * $dim]);
                    $vec_name::from(v)
                }))
            }
        }

        impl std::fmt::Display for $mat_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!(stringify!($mat_name), " ["))?;
                for (i, c) in self.cols.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", c)?;
                }
                write!(f, "]")
            }
        }

        impl From<[[f32; $dim]; $dim]> for $mat_name {
            #[inline(always)]
            fn from(arr: [[f32; $dim]; $dim]) -> Self {
                Self::from_cols(arr.map($vec_name::from))
            }
        }

        impl From<$mat_name> for [$vec_name; $dim] {
            #[inline(always)]
            fn from(m: $mat_name) -> Self {
                m.cols
            }
        }

        impl From<[$vec_name; $dim]> for $mat_name {
            #[inline(always)]
            fn from(cols: [$vec_name; $dim]) -> Self {
                Self { cols }
            }
        }

        impl AsRef<[$vec_name; $dim]> for $mat_name {
            #[inline(always)]
            fn as_ref(&self) -> &[$vec_name; $dim] {
                &self.cols
            }
        }

        impl AsMut<[$vec_name; $dim]> for $mat_name {
            #[inline(always)]
            fn as_mut(&mut self) -> &mut [$vec_name; $dim] {
                &mut self.cols
            }
        }

        impl std::ops::Index<usize> for $mat_name {
            type Output = $vec_name;
            #[inline(always)]
            fn index(&self, i: usize) -> &Self::Output {
                &self.cols[i]
            }
        }

        impl std::ops::IndexMut<usize> for $mat_name {
            #[inline(always)]
            fn index_mut(&mut self, i: usize) -> &mut Self::Output {
                &mut self.cols[i]
            }
        }

        impl std::ops::Mul<$vec_name> for $mat_name {
            type Output = $vec_name;
            #[inline]
            fn mul(self, v: $vec_name) -> $vec_name {
                let v_arr = v.to_array();
                let mut acc = $vec_name::ZERO;
                for i in 0..$dim {
                    acc = acc + self.cols[i] * v_arr[i];
                }
                acc
            }
        }

        impl std::ops::Mul for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn mul(self, other: $mat_name) -> $mat_name {
                $mat_name::from_cols(std::array::from_fn(|i| self * other.cols[i]))
            }
        }

        impl std::ops::Mul<f32> for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn mul(self, s: f32) -> $mat_name {
                let mut m = self;
                for c in m.cols.iter_mut() {
                    *c = *c * s;
                }
                m
            }
        }

        impl std::ops::Div<f32> for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn div(self, s: f32) -> $mat_name {
                self * (1.0 / s)
            }
        }

        impl std::ops::MulAssign<f32> for $mat_name {
            #[inline]
            fn mul_assign(&mut self, s: f32) {
                *self = *self * s;
            }
        }

        impl std::ops::DivAssign<f32> for $mat_name {
            #[inline]
            fn div_assign(&mut self, s: f32) {
                *self = *self / s;
            }
        }

        impl std::ops::Add for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn add(self, other: $mat_name) -> $mat_name {
                $mat_name::from_cols(std::array::from_fn(|i| self.cols[i] + other.cols[i]))
            }
        }

        impl std::ops::AddAssign for $mat_name {
            #[inline]
            fn add_assign(&mut self, other: $mat_name) {
                *self = *self + other
            }
        }

        impl std::ops::Sub for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn sub(self, other: $mat_name) -> $mat_name {
                $mat_name::from_cols(std::array::from_fn(|i| self.cols[i] - other.cols[i]))
            }
        }

        impl std::ops::SubAssign for $mat_name {
            #[inline]
            fn sub_assign(&mut self, other: $mat_name) {
                *self = *self - other
            }
        }

        impl std::ops::Neg for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn neg(self) -> $mat_name {
                $mat_name::from_cols(std::array::from_fn(|i| -self.cols[i]))
            }
        }

        impl std::ops::Div for $mat_name {
            type Output = $mat_name;
            #[inline]
            fn div(self, other: $mat_name) -> $mat_name {
                $mat_name::from_cols(std::array::from_fn(|i| self.cols[i] / other.cols[i]))
            }
        }

        impl std::ops::MulAssign for $mat_name {
            #[inline]
            fn mul_assign(&mut self, other: $mat_name) {
                *self = *self * other;
            }
        }

        impl std::ops::DivAssign for $mat_name {
            #[inline]
            fn div_assign(&mut self, other: $mat_name) {
                *self = *self / other;
            }
        }
    };
}

use define_mat_impls;

macro_rules! define_dense_mat_refs {
    ($mat_name:ident, $dim:expr) => {
        impl AsRef<[[f32; $dim]; $dim]> for $mat_name {
            #[inline(always)]
            fn as_ref(&self) -> &[[f32; $dim]; $dim] {
                // SAFETY: #[repr(C)] with column-major storage of `$dim`` contiguous f32s
                // per column makes `self.cols` bit-identical to `[[f32; $dim]; $dim]`.
                unsafe { &*(self as *const Self as *const [[f32; $dim]; $dim]) }
            }
        }

        impl AsMut<[[f32; $dim]; $dim]> for $mat_name {
            #[inline(always)]
            fn as_mut(&mut self) -> &mut [[f32; $dim]; $dim] {
                unsafe { &mut *(self as *mut Self as *mut [[f32; $dim]; $dim]) }
            }
        }
    };
}

use define_dense_mat_refs;
