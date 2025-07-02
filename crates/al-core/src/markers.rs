mod sealed {
    /// Private base `Marker` trait ensures all required traits are impl'd while users only need to impl their desired mark, the other traits offer distinct use cases
    pub trait Marker:
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
}

#[allow(unused)]
/// `AnyMarker` trait is blanked impl'd for every other marker to act as a generic for <T: AnyMarker> or similar situations
pub trait AnyMarker: sealed::Marker {}
impl<T: sealed::Marker> AnyMarker for T {}

/// `EventMarker` trait acts as a marker for `Event` systems
pub trait EventMarker: sealed::Marker {
    fn _type_name() -> &'static str;
}
impl<T: EventMarker> sealed::Marker for T {}
