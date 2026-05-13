use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
};

#[derive(Clone)]
pub struct NoOpWaker;

impl Wake for NoOpWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

pub fn noop_waker() -> &'static Waker {
    static WAKER: std::sync::LazyLock<Waker> =
        std::sync::LazyLock::new(|| Waker::from(Arc::new(NoOpWaker)));
    &WAKER
}

pub fn noop_context() -> Context<'static> {
    Context::from_waker(noop_waker())
}

pub fn new_noop_waker() -> Waker {
    Waker::from(Arc::new(NoOpWaker))
}
