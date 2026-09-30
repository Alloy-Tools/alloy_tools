use crate::Granularity;
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum NonceType {
    Monotonic,
    MonotonicTimeStamp,
    #[default]
    RandomTimeStamp,
}

/// Context (4 bytes) || Counter (8 bytes) - Simple monotonic counter with ~18.4 quintillion
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Monotonic;

/// Context (4 bytes) || Timestamp (4 bytes) || Counter (4 bytes) - Timestamp monotonic counter with ~4.2 billion
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MonotonicTimeStamp<T: Granularity>(PhantomData<T>);

/// Context (4 bytes) || Timestamp (4 bytes) || Random (4 bytes) - Timestamp with Random
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct RandomTimeStamp<T: Granularity>(PhantomData<T>);
