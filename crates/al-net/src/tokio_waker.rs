use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone, Debug)]
pub struct TokioWaker(Arc<Notify>);

impl TokioWaker {
    pub fn new(notify: Arc<Notify>) -> Self {
        Self(notify)
    }
}

impl std::task::Wake for TokioWaker {
    fn wake(self: Arc<Self>) {
        self.0.notify_one();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.notify_one();
    }
}
