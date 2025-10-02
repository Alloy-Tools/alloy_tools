use std::{any::Any, fmt::Debug, hash::Hash};

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
    'static + Send + Sync + Clone + Default + PartialEq + Any + Debug + Hash
{
}

impl<T: 'static + Send + Sync + Clone + Default + PartialEq + Any + Debug + Hash> EventRequirements
    for T
{
}

/// `EventMarker` trait acts as a marker for `Event` systems and should be derived for each event type
/// It requires impl of `sealed::Marker` to ensure all required traits are impl'd
/// _type_name is derived from the module_path and type name, eg. `my_crate::MyEvent`
pub trait EventMarker: sealed::EventMarker {
    fn _module_path() -> &'static str;
}
impl<T: EventMarker> sealed::EventMarker for T {}

/// Trait marking an item as valid to be a `Transport`
pub trait TransportRequirements: Send + Sync + Debug + Any {}
impl<T: Send + Sync + Debug + Any> TransportRequirements for T {}

/// Trait marking an item as valid for passing through a `Transport`
pub trait TransportItemRequirements: TransportRequirements + Clone {}
impl<T: TransportRequirements + Clone> TransportItemRequirements for T {}

/// `TaskTypes` trait represents the required traits for `Task`s generic types
pub trait TaskTypes: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> TaskTypes for T {}

pub trait TaskStateRequirements: 'static + Send + Sync + Clone {}

impl<T: 'static + Send + Sync + Clone> TaskStateRequirements for T {}
