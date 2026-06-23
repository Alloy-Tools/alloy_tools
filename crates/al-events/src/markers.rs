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
pub trait ObjectTraits: Send + Sync + Debug + Any + 'static {}

impl<T: Send + Sync + Debug + Any + 'static> ObjectTraits for T {}

/// Required traits for an event type to be used in the event system
pub trait MessageRequirements:
    'static + Send + Sync + Clone + Default + PartialEq + Any + Debug + Hash + sealed::SerdeFeature
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
            + sealed::SerdeFeature,
    > MessageRequirements for T
{
}

/// `MessageMarker` trait acts as a marker for `Message` systems and should be derived for each message type.
/// It requires an impl of `sealed::MessageMarker` to ensure all required traits are impl'd
/// type_with_generics is derived from the module_path and type name, eg. `my_crate::MyMessage`.
/// Generics are included in simple name form through the `tynm` crate. This is used for type registration and lookup.
pub trait MessageMarker: sealed::MessageMarker {
    fn module_path() -> &'static str;
    /// Helper function to return the simple names of generic messages
    fn type_with_generics() -> String {
        format!("{}::{}", Self::module_path(), tynm::type_name::<Self>())
    }
}
impl<T: MessageMarker> sealed::MessageMarker for T {}
