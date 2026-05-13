use std::{collections::VecDeque, task::Waker};

use crate::{Transport, TransportItemRequirements};

pub struct Filter<T, F> {
    predicate: F,
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T, F> Filter<T, F> {
    pub fn new(predicate: F) -> Self {
        Self {
            predicate,
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements, F: Fn(&T) -> bool + Send + 'static> Transport<T>
    for Filter<T, F>
{
    /// Receive incoming data and filter it based on the predicate function.
    ///
    /// # Panics
    ///
    /// If the predicate function panics, the calling code will panic and the data item will be lost.
    /// Ensure your predicate is panic-free, or catch panics at a higher level if filtering may fail.
    fn handle_incoming(&mut self, data: T) {
        if (self.predicate)(&data) {
            self.queue.push_back(data);
            if let Some(w) = self.waker.take() {
                w.wake();
            }
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
        format!("Filter Buffer Length: {}", self.queue.len())
    }
}
