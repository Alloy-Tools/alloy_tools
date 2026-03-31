pub mod sealed {
    pub trait Sealed {}
}

// Marker for `ComponentTrait` super traits
pub trait ComponentTraitRequirements:
    Send + Sync + std::any::Any + std::fmt::Debug + erased_serde::Serialize + sealed::Sealed
{
}
impl<T: Send + Sync + std::any::Any + std::fmt::Debug + erased_serde::Serialize + for<'de> serde::Deserialize<'de>> sealed::Sealed
    for T
{
}
impl<
        T: Send + Sync + std::any::Any + std::fmt::Debug + erased_serde::Serialize + for<'de> serde::Deserialize<'de> + sealed::Sealed,
    > ComponentTraitRequirements for T
{
}

// Marker for `Component` required traits
pub trait ComponentRequirements:
    ComponentTraitRequirements + Clone + PartialEq + serde::Serialize + for<'de> serde::Deserialize<'de> + sealed::Sealed
{
}
impl<
        T: ComponentTraitRequirements
            + Clone
            + PartialEq
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>
            + sealed::Sealed,
    > ComponentRequirements for T
{
}
