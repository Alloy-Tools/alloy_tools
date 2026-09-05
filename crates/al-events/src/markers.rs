use al_structures::traits::{DynTypeName, TypeName};
use std::{any::Any, fmt::Debug, hash::Hash};

/// Sealed marker traits ensure all required traits are impl'd while users can only impl their desired mark
mod sealed {
    pub trait MessageMarker: super::MessageRequirements {}

    /// Used to mark other code that have required traits as valid for serde features
    /// If no serde feature, this is a dummy trait
    #[cfg(not(feature = "serde"))]
    pub trait SerdeFeature {}

    /// If no serde feature, all EventMarker types implement this dummy trait
    #[cfg(not(feature = "serde"))]
    impl<T: MessageMarker> SerdeFeature for T {}

    /// If serde feature is enabled, this requires erased_serde Serialize
    #[cfg(feature = "serde")]
    pub trait SerdeFeature: erased_serde::Serialize {}
    /// If serde feature is enabled, all EventMarker types that also implement serde::Serialize implement this trait
    #[cfg(feature = "serde")]
    impl<T: serde::Serialize + for<'de> serde::Deserialize<'de>> SerdeFeature for T {}
}

/// Shared traits required for Command, Event, and Query traits
pub trait ObjectTraits: Send + Sync + Debug + Any + DynTypeName + 'static {}

impl<T: Send + Sync + Debug + Any + DynTypeName + 'static> ObjectTraits for T {}

//REVIEW: Do I really need Default and Hash? Transport only needs `Debug + Clone + Send`
/// Required traits for a message type to be used in the message system
pub trait MessageRequirements:
    'static
    + Send
    + Sync
    + Clone
    + Default
    + PartialEq
    + Any
    + Debug
    + Hash
    + sealed::SerdeFeature
    + TypeName
{
}

impl<
        T: 'static
            + Send
            + Sync
            + Clone
            + Default
            + PartialEq
            + Any
            + Debug
            + Hash
            + sealed::SerdeFeature
            + TypeName,
    > MessageRequirements for T
{
}

/// `MessageMarker` trait acts as a marker for `Message` systems and should be derived for each message type.
pub trait MessageMarker: sealed::MessageMarker {}
impl<T: MessageMarker> sealed::MessageMarker for T {}
