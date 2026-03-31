pub trait NumUtils {
    fn sqrt(self) -> f64;

    fn is_nan(&self) -> bool;

    fn abs_value(self) -> Self;

    fn min(self, other: Self) -> Self;

    fn max(self, other: Self) -> Self;

    fn saturating_sub(self, other: Self) -> Self;

    fn saturating_add(self, other: Self) -> Self;
}

macro_rules! impl_num_utils_for_uint {
    ($($t:ty),*) => {
        $(impl NumUtils for $t {
            #[inline]
            fn sqrt(self) -> f64 {
                <$t as num_traits::ToPrimitive>::to_f64(&self).unwrap_or_else(num_traits::zero)
            }

            #[inline]
            fn is_nan(&self) -> bool {
                false
            }

            #[inline]
            fn abs_value(self) -> Self {
                self
            }

            #[inline]
            fn min(self, other: Self) -> Self {
                <$t as Ord>::min(self, other)
            }

            #[inline]
            fn max(self, other: Self) -> Self {
                <$t as Ord>::max(self, other)
            }

            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                <$t>::saturating_sub(self, other)
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                <$t>::saturating_add(self, other)
            }
        })*
    };
}

macro_rules! impl_num_utils_for_iint {
    ($($t:ty),*) => {
        $(impl NumUtils for $t {
            #[inline]
            fn sqrt(self) -> f64 {
                <$t as num_traits::ToPrimitive>::to_f64(&self).unwrap_or_else(num_traits::zero)
            }

            #[inline]
            fn is_nan(&self) -> bool {
                false
            }

            #[inline]
            fn abs_value(self) -> Self {
                if self < 0 {
                    -self
                } else {
                    self
                }
            }

            #[inline]
            fn min(self, other: Self) -> Self {
                <$t as Ord>::min(self, other)
            }

            #[inline]
            fn max(self, other: Self) -> Self {
                <$t as Ord>::max(self, other)
            }

            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                <$t>::saturating_sub(self, other)
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                <$t>::saturating_add(self, other)
            }
        })*
    };
}

macro_rules! impl_num_utils_for_float {
    ($($t:ty),*) => {
        $(impl NumUtils for $t {
            #[inline]
            fn sqrt(self) -> f64 {
                self.sqrt() as f64
            }

            #[inline]
            fn is_nan(&self) -> bool {
                <$t>::is_nan(*self)
            }

            #[inline]
            fn abs_value(self) -> Self {
                if self < 0.0 {
                    -self
                } else {
                    self
                }
            }

            fn min(self, other: Self) -> Self {
                match (self.is_nan(), other.is_nan()) {
                    (true, true) => self,   // both are NaN, return NaN
                    (true, false) => other, // self is NaN, return other
                    (false, true) => self,  // other is NaN, return self
                    (false, false) => {
                        if self <= other {
                            self
                        } else {
                            other
                        }
                    }
                }
            }

            fn max(self, other: Self) -> Self {
                match (self.is_nan(), other.is_nan()) {
                    (true, true) => self,   // both are NaN, return NaN
                    (true, false) => other, // self is NaN, return other
                    (false, true) => self,  // other is NaN, return self
                    (false, false) => {
                        if self >= other {
                            self
                        } else {
                            other
                        }
                    }
                }
            }

            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                (self - other).max(<$t>::MIN).min(<$t>::MAX)
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                (self - other).max(<$t>::MIN).min(<$t>::MAX)
            }
        })*
    };
}

impl_num_utils_for_iint!(i8, i16, i32, i64, i128, isize);
impl_num_utils_for_uint!(u8, u16, u32, u64, u128, usize);
impl_num_utils_for_float!(f32, f64);
