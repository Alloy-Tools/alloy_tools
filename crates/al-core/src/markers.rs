use std::{any::Any, fmt::Debug};

// Marker trait for partitioning
pub trait Marker: 'static + Any + Send + Sync + Clone + Default + PartialEq + Debug {}

pub trait EventMarker: Marker {}
impl<T: EventMarker> Marker for T {}
