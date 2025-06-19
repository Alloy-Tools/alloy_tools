mod sealed {
    /// private base `Marker` trait ensures all required traits are impl'd while users only need to impl their desired mark, the other traits offer distinct use cases
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

/// `EventMarker` trait acts as a marker for `Event` systems
pub trait EventMarker: sealed::Marker {}
impl<T: EventMarker> sealed::Marker for T {}
