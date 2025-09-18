/// Sealed marker traits ensure all required traits are impl'd while users can only impl their desired mark
mod sealed {
    pub trait EventMarker:
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

/// `EventMarker` trait acts as a marker for `Event` systems and should be derived for each event type
/// It requires impl of `sealed::Marker` to ensure all required traits are impl'd
/// _type_name is derived from the module_path and type name, eg. `my_crate::MyEvent`
pub trait EventMarker: sealed::EventMarker {
    fn _type_name() -> &'static str;
}
impl<T: EventMarker> sealed::EventMarker for T {}