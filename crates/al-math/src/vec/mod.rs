mod vec2;
mod vec3;
mod vec4;

pub use vec2::Vec2;
pub use vec3::{Vec3, Vec3A};
pub use vec4::Vec4;

macro_rules! define_vec_impls {
    (
        $struct_name:ident,
        $dim:expr,
        $($field:ident),+ $(; $pad:ident)?
    ) => {
        impl $struct_name {
            pub const ZERO: Self = Self {
                $($field: 0.0,)+
                $($pad: 0.0,)?
            };
            pub const ONE: Self = Self {
                $($field: 1.0,)+
                $($pad: 0.0,)?
            };

            #[inline(always)]
            pub fn new($($field: f32),+) -> Self {
                Self {
                    $($field,)+
                    $($pad: 0.0,)?
                }
            }

            #[must_use]
            #[inline(always)]
            pub fn to_array(self) -> [f32; $dim] {
                [$(self.$field),+]
            }

            #[inline(always)]
            pub fn from_array(arr: [f32; $dim]) -> Self {
                let [$($field),+] = arr;
                Self {
                    $($field,)+
                    $($pad: 0.0,)?
                }
            }

            #[inline]
            pub fn mul_add(self, a: Self, b: Self) -> Self {
                Self::new($(self.$field.mul_add(a.$field, b.$field)),+)
            }

            #[must_use]
            #[inline]
            pub fn dot(self, other: Self) -> f32 {
                0. $(+ (self.$field * other.$field))+
            }

            #[must_use]
            #[inline]
            pub fn length_squared(self) -> f32 {
                0. $(+ (self.$field * self.$field))+
            }

            #[must_use]
            #[inline]
            pub fn length(self) -> f32 {
                self.length_squared().sqrt()
            }

            #[inline]
            pub fn normalize(self) -> Self {
                let len = self.length();
                if len == 0. || len.is_nan() {
                    Self::ZERO
                } else {
                    self * (1. / len)
                }
            }

            #[must_use]
            #[inline]
            pub fn distance_squared(self, other: Self) -> f32 {
                (self - other).length_squared()
            }

            #[must_use]
            #[inline]
            pub fn distance(self, other: Self) -> f32 {
                (self - other).length()
            }

            #[inline]
            pub fn lerp(self, other: Self, t: f32) -> Self {
                self * (1. - t) + other * t
            }

            #[inline]
            pub fn min(self, other: Self) -> Self {
                Self::new($(self.$field.min(other.$field)),+)
            }

            #[inline]
            pub fn max(self, other: Self) -> Self {
                Self::new($(self.$field.max(other.$field)),+)
            }

            #[inline]
            pub fn abs(self) -> Self {
                Self::new($(self.$field.abs()),+)
            }

            #[inline]
            pub fn clamp(self, lo: Self, hi: Self) -> Self {
                self.max(lo).min(hi)
            }

            #[must_use]
            #[inline]
            pub fn is_finite(self) -> bool {
                true $(&& self.$field.is_finite())+
            }

            #[must_use]
            #[inline]
            pub fn approx_eq(self, other: Self, eps: f32) -> bool {
                true $(&& (self.$field - other.$field).abs() <= eps)+
            }
        }

        impl IntoIterator for $struct_name {
            type Item = f32;
            type IntoIter = std::array::IntoIter<f32, $dim>;
            fn into_iter(self) -> Self::IntoIter { self.to_array().into_iter() }
        }

        impl std::iter::Sum for $struct_name {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::ZERO, |a, b| a + b)
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {{", stringify!($struct_name))?;
                let mut first = true;
                $(
                    if !first { write!(f, ", ")?; }
                    first = false;
                    write!(f, "{}: {}", stringify!($field), self.$field)?;
                )+
                write!(f, " }}")
            }
        }

        impl From<[f32; $dim]> for $struct_name {
            #[inline]
            fn from(arr: [f32; $dim]) -> Self {
                Self::from_array(arr)
            }
        }

        impl From<$struct_name> for [f32; $dim] {
            #[inline]
            fn from(v: $struct_name) -> Self {
                v.to_array()
            }
        }

        impl AsRef<[f32; $dim]> for $struct_name {
            #[inline]
            fn as_ref(&self) -> &[f32; $dim] {
                // SAFETY: The struct is repr(C) and contains only f32 fields.
                unsafe { &*(self as *const Self as *const [f32; $dim]) }
            }
        }

        impl AsMut<[f32; $dim]> for $struct_name {
            #[inline]
            fn as_mut(&mut self) -> &mut [f32; $dim] {
                // SAFETY: The struct is repr(C) and contains only f32 fields.
                unsafe { &mut *(self as *mut Self as *mut [f32; $dim]) }
            }
        }

        impl PartialEq for $struct_name {
            fn eq(&self, other: &Self) -> bool {
                true $(&& (self.$field == other.$field || (self.$field.is_nan() && other.$field.is_nan())))+
            }
        }
        impl Eq for $struct_name {}

        impl std::hash::Hash for $struct_name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                fn normalize(val: f32) -> f32 {
                    if val == 0. {
                        0.
                    } else if val.is_nan() {
                        f32::NAN
                    } else {
                        val
                    }
                }
                $(normalize(self.$field).to_bits().hash(state);)+
            }
        }

        impl PartialOrd for $struct_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $struct_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                use std::cmp::Ordering;
                fn cmp_f(a: f32, b: f32) -> Ordering {
                    match (a.is_nan(), b.is_nan()) {
                        (true, true) => Ordering::Equal,
                        (true, false) => Ordering::Greater,
                        (false, true) => Ordering::Less,
                        (false, false) => a.partial_cmp(&b).expect("All other cases matched"),
                    }
                }

                Ordering::Equal
                    $(.then_with(|| cmp_f(self.$field, other.$field)))+
            }
        }

        impl std::ops::Neg for $struct_name {
            type Output = Self;

            fn neg(mut self) -> Self::Output {
                $(self.$field = -self.$field;)+
                self
            }
        }

        impl std::ops::Add for $struct_name {
            type Output = Self;
            #[inline]
            fn add(self, other: Self) -> Self::Output {
                Self::new($(self.$field + other.$field),+)
            }
        }

        impl std::ops::AddAssign for $struct_name {
            #[inline]
            fn add_assign(&mut self, other: Self) {
                *self = *self + other
            }
        }

        impl std::ops::Sub for $struct_name {
            type Output = Self;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                Self::new($(self.$field - other.$field),+)
            }
        }

        impl std::ops::SubAssign for $struct_name {
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                *self = *self - other
            }
        }

        impl std::ops::Mul<f32> for $struct_name {
            type Output = Self;
            #[inline]
            fn mul(self, scalar: f32) -> Self::Output {
                Self::new($(self.$field * scalar),+)
            }
        }

        impl std::ops::MulAssign<f32> for $struct_name {
            #[inline]
            fn mul_assign(&mut self, scalar: f32) {
                *self = *self * scalar
            }
        }

        impl std::ops::Mul<$struct_name> for f32 {
            type Output = $struct_name;
            #[inline]
            fn mul(self, vec: $struct_name) -> Self::Output {
                vec * self
            }
        }

        impl std::ops::Mul for $struct_name {
            type Output = Self;
            #[inline]
            fn mul(self, other: Self) -> Self::Output {
                Self::new($(self.$field * other.$field),+)
            }
        }

        impl std::ops::MulAssign for $struct_name {
            #[inline]
            fn mul_assign(&mut self, other: Self) {
                *self = *self * other
            }
        }

        impl std::ops::Div<f32> for $struct_name {
            type Output = Self;
            #[inline]
            fn div(self, scalar: f32) -> Self::Output {
                let reciprocal = 1.0 / scalar;
                Self::new($(self.$field * reciprocal),+)
            }
        }

        impl std::ops::DivAssign<f32> for $struct_name {
            #[inline]
            fn div_assign(&mut self, scalar: f32) {
                *self = *self / scalar
            }
        }

        impl std::ops::Div for $struct_name {
            type Output = Self;
            #[inline]
            fn div(self, other: Self) -> Self::Output {
                Self::new($(self.$field / other.$field),+)
            }
        }

        impl std::ops::DivAssign for $struct_name {
            #[inline]
            fn div_assign(&mut self, other: Self) {
                *self = *self / other
            }
        }
    };
}

use define_vec_impls;
