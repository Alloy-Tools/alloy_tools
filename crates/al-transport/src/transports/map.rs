use std::{collections::VecDeque, task::Waker};

use crate::{Transport, TransportItemRequirements};

pub struct Map<T, F> {
    f: F,
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T, F> Map<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + 'static> From<Map<T, F>>
    for Box<dyn Transport<T>>
{
    fn from(value: Map<T, F>) -> Self {
        Box::new(value)
    }
}

impl<T: TransportItemRequirements, F: Fn(T) -> T + Send + 'static> Transport<T> for Map<T, F> {
    /// Receive incoming data and apply the transformation function.
    ///
    /// # Panics
    ///
    /// If the transformation function `f` panics, the calling code will panic and the data item
    /// will be lost. Ensure your transformation function is panic-free, or catch panics at a
    /// higher level if transformations may fail.
    fn handle_incoming(&mut self, data: T) {
        self.queue.push_back((self.f)(data));
        if let Some(w) = self.waker.take() {
            w.wake();
        }
    }

    fn poll_action(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<crate::Action<T>> {
        if let Some(data) = self.queue.pop_front() {
            std::task::Poll::Ready(crate::Action::Data(data))
        } else {
            self.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }

    fn status(&self) -> String {
        format!("Map Buffer Length: {}", self.queue.len())
    }
}
