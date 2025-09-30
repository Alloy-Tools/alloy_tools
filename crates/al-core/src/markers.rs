/// Sealed marker traits ensure all required traits are impl'd while users can only impl their desired mark
mod sealed {
    pub trait EventMarker: super::EventRequirements {}

    /// Used to mark other code that have required traits as valid for serde features
    /// If no serde feature, this is a dummy trait
    #[cfg(not(feature = "serde"))]
    pub trait SerdeFeature {}
    /// If no serde feature, all EventMarker types implement this dummy trait
    #[cfg(not(feature = "serde"))]
    impl<T: EventMarker> SerdeFeature for T {}

    /// If serde feature is enabled, this requires erased_serde Serialize
    #[cfg(feature = "serde")]
    pub trait SerdeFeature: erased_serde::Serialize {}
    /// If serde feature is enabled, all EventMarker types that also implement serde::Serialize implement this trait
    #[cfg(feature = "serde")]
    impl<
            T: EventMarker
                + serde::Serialize
                + for<'de> serde::Deserialize<'de>
                + for<'de> serde::de::Deserialize<'de>,
        > SerdeFeature for T
    {
    }
}

pub trait SerdeFeature: sealed::SerdeFeature {}
impl<T: sealed::SerdeFeature> SerdeFeature for T {}

/// Required traits for an event type to be used in the event system
pub trait EventRequirements:
    'static
    + Send
    + Sync
    + Clone
    + Default
    + PartialEq
    + std::any::Any
    + std::fmt::Debug
    + std::hash::Hash
{
}

impl<
        T: 'static
            + Send
            + Sync
            + Clone
            + Default
            + PartialEq
            + std::any::Any
            + std::fmt::Debug
            + std::hash::Hash,
    > EventRequirements for T
{
}

/// `EventMarker` trait acts as a marker for `Event` systems and should be derived for each event type
/// It requires impl of `sealed::Marker` to ensure all required traits are impl'd
/// _type_name is derived from the module_path and type name, eg. `my_crate::MyEvent`
pub trait EventMarker: sealed::EventMarker {
    fn _module_path() -> &'static str;
}
impl<T: EventMarker> sealed::EventMarker for T {}

pub trait TransportRequirements: Send + Sync + std::fmt::Debug + std::any::Any {}
impl<T: Send + Sync + std::fmt::Debug + std::any::Any> TransportRequirements for T {}
