use al_structures::traits::CloneEqError;
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
impl std::error::Error for TransportIDError {}

pub enum Action<T: TransportItemRequirements> {
    /// Transport has data to push downstream.
    Data(T),
    /// Transport is idle; driver should try again later.
    Pending,
    /// Transport has failed terminally. The driver should remove it.
    Error(TransportError),
}

/// Pure Sans‑IO state machine. Never blocks, never does I/O.
pub trait Transport<T: TransportItemRequirements>: Send + 'static {
    /// Feed data from upstream.
    fn handle_incoming(&mut self, data: T) -> Result<(), Backpressure>;

    /// Try to produce a downstream item.
    fn poll_action(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Action<T>>;

    fn has_space(&self) -> Vacancy;

    fn status(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vacancy {
    /// Unbounded capacity, unlimited vacancy.
    Unbounded,
    /// Room for at least this many more items.
    Vacancy(usize),
    /// No room, delegate to backpressure.
    Backpressure(Backpressure),
}

impl Vacancy {
    pub fn is_available(&self) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Vacancy(n) => *n > 0,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backpressure {
    /// Input buffer full. Retry later.
    BufferFull,
    /// Transport has closed. Stop feeding it.
    Closed,
}

impl std::fmt::Display for Backpressure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backpressure::BufferFull => write!(f, "Buffer is full, try again later."),
            Backpressure::Closed => write!(f, "Transport is closed."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Backpressure(Backpressure),
    Custom(Box<dyn CloneEqError>),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Backpressure(err) => write!(f, "{err}"),
            TransportError::Custom(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TransportError::Custom(err) => err.source(),
            _ => None,
        }
    }
}
