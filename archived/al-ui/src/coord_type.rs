mod sealed {
    pub trait Sealed {}
}

/// Coordinate type trait
pub trait CoordType:
    'static
    + Send
    + Sync
    + Copy
    + PartialOrd
    + std::fmt::Debug
    + crate::NumUtils
    + num_traits::Num
    + num_traits::NumAssign
    + num_traits::Bounded
    + num_traits::FromPrimitive
    + num_traits::ToPrimitive
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + sealed::Sealed
{
}
impl<
        T: 'static
            + Send
            + Sync
            + Copy
            + PartialOrd
            + std::fmt::Debug
            + crate::NumUtils
            + num_traits::Num
            + num_traits::NumAssign
            + num_traits::Bounded
            + num_traits::FromPrimitive
            + num_traits::ToPrimitive
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>,
    > sealed::Sealed for T
{
}
impl<
        T: 'static
            + Send
            + Sync
            + Copy
            + PartialOrd
            + std::fmt::Debug
            + crate::NumUtils
            + num_traits::Num
            + num_traits::NumAssign
            + num_traits::Bounded
            + num_traits::FromPrimitive
            + num_traits::ToPrimitive
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>
            + sealed::Sealed,
    > CoordType for T
{
}
