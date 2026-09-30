pub trait TransportItemRequirements: std::fmt::Debug + Clone + Send + 'static {}
impl<T: std::fmt::Debug + Clone + Send + 'static> TransportItemRequirements for T {}