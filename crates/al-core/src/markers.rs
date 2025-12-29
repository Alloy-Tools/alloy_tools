#[cfg(any(feature = "event", feature = "transport"))]
use std::fmt::Debug;
#[cfg(feature = "event")]
use std::{any::Any, hash::Hash};

#[cfg(feature = "event")]
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
#[cfg(feature = "event")]
pub trait SerdeFeature: sealed::SerdeFeature {}
#[cfg(feature = "event")]
impl<T: sealed::SerdeFeature> SerdeFeature for T {}

#[cfg(feature = "event")]
/// Required traits for an event type to be used in the event system
pub trait EventRequirements:
    'static + Send + Sync + Clone + Default + PartialEq + Any + Debug + Hash
{
}

#[cfg(feature = "event")]
impl<T: 'static + Send + Sync + Clone + Default + PartialEq + Any + Debug + Hash> EventRequirements
    for T
{
}

#[cfg(feature = "event")]
/// `EventMarker` trait acts as a marker for `Event` systems and should be derived for each event type
/// It requires impl of `sealed::Marker` to ensure all required traits are impl'd
/// _type_name is derived from the module_path and type name, eg. `my_crate::MyEvent`. Generics are included in simple name form through the `tynm` crate. This is used for event registration and lookup.
pub trait EventMarker: sealed::EventMarker {
    fn module_path() -> &'static str;
    /// Helper function to return the simple names of generic events
    fn type_with_generics() -> String {
        format!("{}::{}", Self::module_path(), tynm::type_name::<Self>())
    }
}
#[cfg(feature = "event")]
impl<T: EventMarker> sealed::EventMarker for T {}

#[cfg(feature = "transport")]
/// Trait marking an item as valid to be a `Transport`
pub trait TransportRequirements: 'static + Send + Sync + Debug + Any {
    fn as_any(&self) -> &dyn std::any::Any;
}
#[cfg(feature = "transport")]
impl<T: 'static + Send + Sync + Debug + Any> TransportRequirements for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "transport")]
/// Trait marking an item as valid for passing through a `Transport`
pub trait TransportItemRequirements: TransportRequirements + Clone {}
#[cfg(feature = "transport")]
impl<T: TransportRequirements + Clone> TransportItemRequirements for T {}

#[cfg(feature = "task")]
/// `TaskTypes` trait represents the required traits for `Task`s generic types
pub trait TaskTypes: Send + Sync + Clone + 'static {}
#[cfg(feature = "task")]
impl<T: Send + Sync + Clone + 'static> TaskTypes for T {}

#[cfg(feature = "task")]
pub trait TaskStateRequirements: 'static + Send + Sync + Clone {}
#[cfg(feature = "task")]
impl<T: 'static + Send + Sync + Clone> TaskStateRequirements for T {}
