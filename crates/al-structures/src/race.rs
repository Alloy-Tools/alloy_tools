//! Utility for racing two futures and returning the first one to complete.

use crate::enums::Which;
use std::{future::Future, task::Poll};

/// Races two boxed futures. Resolves with the winner's output.
pub struct Race<'a, A, B> {
    a: std::pin::Pin<Box<dyn Future<Output = A> + Send + 'a>>,
    b: std::pin::Pin<Box<dyn Future<Output = B> + Send + 'a>>,
}

impl<'a, A, B> Race<'a, A, B> {
    /// Creates a new race helper for the given pair of futures.
    ///
    /// When polled, this future will poll both futures and return `Which::A` or `Which::B`
    /// depending on which completes first.
    ///
    /// # Examples
    ///
    /// ```
    /// use al_structures::{race::Race, enums::Which, noop_waker::noop_context};
    /// use std::{pin::Pin, future::Future, task::Poll};
    ///
    /// let a = Box::pin(async { 42 });
    /// let b = Box::pin(async { "hello" });
    /// let mut race = Race::new(a, b);
    /// let mut cx = noop_context();
    ///
    /// // A is returned as it is polled first
    /// match Pin::new(&mut race).poll(&mut cx) {
    ///     Poll::Ready(Which::A(42)) => {}
    ///     _ => panic!("unexpected result"),
    /// }
    /// ```
    pub fn new(
        a: std::pin::Pin<Box<dyn Future<Output = A> + Send + 'a>>,
        b: std::pin::Pin<Box<dyn Future<Output = B> + Send + 'a>>,
    ) -> Self {
        Race { a, b }
    }
}

impl<'a, A, B> Future for Race<'a, A, B> {
    type Output = Which<A, B>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();

        // poll  a
        if let Poll::Ready(val) = this.a.as_mut().poll(cx) {
            return Poll::Ready(Which::A(val));
        }

        // poll b
        if let Poll::Ready(val) = this.b.as_mut().poll(cx) {
            return Poll::Ready(Which::B(val));
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noop_waker::noop_context;
    use std::{pin::Pin, task::Poll};

    #[test]
    fn a_wins() {
        let mut cx = noop_context();

        let a = Box::pin(async { 42 });
        let b = Box::pin(async { "unreached" });
        let mut race = Race::new(a, b);

        // a is ready immediately
        match Pin::new(&mut race).poll(&mut cx) {
            Poll::Ready(Which::A(42)) => {}
            other => panic!("expected Which::A(42), got {:?}", other),
        }
    }

    #[test]
    fn b_wins() {
        let mut cx = noop_context();

        let a = Box::pin(async { std::future::pending::<u8>().await });
        let b = Box::pin(async { "winner" });
        let mut race = Race::new(a, b);

        // b is ready after a polls as pending
        match Pin::new(&mut race).poll(&mut cx) {
            Poll::Ready(Which::B("winner")) => {}
            other => panic!("expected Which::B(\"winner\"), got {:?}", other),
        }
    }

    #[test]
    fn both_pending() {
        let mut cx = noop_context();

        let a = Box::pin(async { std::future::pending::<u8>().await });
        let b = Box::pin(async { std::future::pending::<&str>().await });
        let mut race = Race::new(a, b);

        match Pin::new(&mut race).poll(&mut cx) {
            Poll::Pending => {}
            other => panic!("expected Poll::Pending, got {:?}", other),
        }
    }
}
