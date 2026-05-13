use crate::{Transport, TransportItemRequirements};
use std::{collections::VecDeque, task::Waker};

pub struct Queue<T> {
    queue: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T: TransportItemRequirements> From<Queue<T>> for Box<dyn Transport<T>> {
    fn from(value: Queue<T>) -> Self {
        Box::new(value)
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T: TransportItemRequirements> Transport<T> for Queue<T> {
    fn handle_incoming(&mut self, data: T) {
        self.queue.push_back(data);
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
        format!("Queue Length: {}", self.queue.len())
    }
}
