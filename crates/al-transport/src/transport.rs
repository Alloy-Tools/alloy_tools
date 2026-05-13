use crate::TransportItemRequirements;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TransportID {
    pub(crate) index: usize,
    pub(crate) generation: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportIDError {
    InvalidIndex,
    InvalidGeneration,
}

impl std::fmt::Display for TransportIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &TransportIDError::InvalidIndex => write!(f, "Invalid index for TransportID"),
            TransportIDError::InvalidGeneration => write!(f, "Invalid generation for TransportID"),
        }
    }
}

pub enum Action<T: TransportItemRequirements> {
    /// Transport has data to push downstream.
    Data(T),
    /// Transport is idle; driver should try again later.
    Pending,
}

/// Pure Sans‑IO state machine. Never blocks, never does I/O.
pub trait Transport<T: TransportItemRequirements>: Send + 'static {
    /// Feed data from upstream.
    fn handle_incoming(&mut self, data: T);

    /// Try to produce a downstream item.
    fn poll_action(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Action<T>>;

    fn status(&self) -> String;
}
