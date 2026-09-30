pub mod nonce;
pub mod nonce_counter;
pub mod nonce_timestamp;
pub mod nonce_trait;
pub mod nonce_type;

mod sealed {
    use crate::Granularity;

    pub trait Sealed {}
    impl Sealed for super::nonce_type::Monotonic {}
    impl<G: Granularity> Sealed for super::nonce_type::MonotonicTimeStamp<G> {}
    impl<G: Granularity> Sealed for super::nonce_type::RandomTimeStamp<G> {}
}
