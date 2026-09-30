use crate::{Action, Backpressure, Transport, TransportItemRequirements, Vacancy};
use std::{
    marker::PhantomData,
    task::{Context, Poll},
};

pub struct Sink<T, F> {
    f: F,
    _p: PhantomData<fn(T)>,
}

impl<T, F> Sink<T, F> {
    pub fn new(f: F) -> Self {
        Self { f, _p: PhantomData }
    }
}

impl<T: TransportItemRequirements, F: FnMut(T) + Send + 'static> From<Sink<T, F>>
    for Box<dyn Transport<T>>
{
    fn from(value: Sink<T, F>) -> Self {
        Box::new(value)
    }
}

impl<T: TransportItemRequirements, F: FnMut(T) + Send + 'static> Transport<T> for Sink<T, F> {
    fn handle_incoming(&mut self, data: T) -> Result<(), Backpressure> {
        (self.f)(data);
        Ok(())
    }

    fn poll_action(&mut self, _: &mut Context<'_>) -> Poll<Action<T>> {
        Poll::Pending
    }

    fn has_space(&self) -> Vacancy {
        Vacancy::Unbounded
    }

    fn status(&self) -> String {
        "Sink".to_string()
    }
}

// ----- Fallible Sink -----
pub struct FallibleSink<T, F> {
    f: F,
    _p: PhantomData<fn(T)>,
}

impl<T, F> FallibleSink<T, F> {
    pub fn new(f: F) -> Self {
        Self { f, _p: PhantomData }
    }
}

impl<T: TransportItemRequirements, F: FnMut(T) -> Result<(), Backpressure> + Send + 'static>
    From<FallibleSink<T, F>> for Box<dyn Transport<T>>
{
    fn from(value: FallibleSink<T, F>) -> Self {
        Box::new(value)
    }
}

impl<T: TransportItemRequirements, F: FnMut(T) -> Result<(), Backpressure> + Send + 'static>
    Transport<T> for FallibleSink<T, F>
{
    fn handle_incoming(&mut self, data: T) -> Result<(), Backpressure> {
        (self.f)(data)
    }

    fn poll_action(&mut self, _: &mut Context<'_>) -> Poll<Action<T>> {
        Poll::Pending
    }

    fn has_space(&self) -> Vacancy {
        Vacancy::Unbounded
    }

    fn status(&self) -> String {
        "FallibleSink".to_string()
    }
}

// ----- Stateful Sink -----
pub struct StatefulSink<T, S, F> {
    state: S,
    f: F,
    _p: PhantomData<fn(T)>,
}

impl<T, S, F> StatefulSink<T, S, F> {
    pub fn new(state: S, f: F) -> Self {
        Self {
            state,
            f,
            _p: PhantomData,
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<
        T: TransportItemRequirements,
        S: std::fmt::Display + Send + 'static,
        F: FnMut(&mut S, T) + Send + 'static,
    > From<StatefulSink<T, S, F>> for Box<dyn Transport<T>>
{
    fn from(value: StatefulSink<T, S, F>) -> Self {
        Box::new(value)
    }
}

impl<
        T: TransportItemRequirements,
        S: std::fmt::Display + Send + 'static,
        F: FnMut(&mut S, T) + Send + 'static,
    > Transport<T> for StatefulSink<T, S, F>
{
    fn handle_incoming(&mut self, data: T) -> Result<(), Backpressure> {
        (self.f)(&mut self.state, data);
        Ok(())
    }

    fn poll_action(&mut self, _: &mut Context<'_>) -> Poll<Action<T>> {
        Poll::Pending
    }

    fn has_space(&self) -> Vacancy {
        Vacancy::Unbounded
    }

    fn status(&self) -> String {
        format!("Stateful Sink {{ state: {} }}", self.state)
    }
}

// ----- Fallible Stateful Sink -----
pub struct FallibleStatefulSink<T, S, F> {
    state: S,
    f: F,
    _p: PhantomData<fn(T)>,
}

impl<T, S, F> FallibleStatefulSink<T, S, F> {
    pub fn new(state: S, f: F) -> Self {
        Self {
            state,
            f,
            _p: PhantomData,
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<
        T: TransportItemRequirements,
        S: std::fmt::Display + Send + 'static,
        F: FnMut(&mut S, T) -> Result<(), Backpressure> + Send + 'static,
    > From<FallibleStatefulSink<T, S, F>> for Box<dyn Transport<T>>
{
    fn from(value: FallibleStatefulSink<T, S, F>) -> Self {
        Box::new(value)
    }
}

impl<
        T: TransportItemRequirements,
        S: std::fmt::Display + Send + 'static,
        F: FnMut(&mut S, T) -> Result<(), Backpressure> + Send + 'static,
    > Transport<T> for FallibleStatefulSink<T, S, F>
{
    fn handle_incoming(&mut self, data: T) -> Result<(), Backpressure> {
        (self.f)(&mut self.state, data)
    }

    fn poll_action(&mut self, _: &mut Context<'_>) -> Poll<Action<T>> {
        Poll::Pending
    }

    fn has_space(&self) -> Vacancy {
        Vacancy::Unbounded
    }

    fn status(&self) -> String {
        format!("Stateful Sink {{ state: {} }}", self.state)
    }
}
