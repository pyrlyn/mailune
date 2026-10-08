//! Test helpers shared by the modules of this crate.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Polls `future` once. Every engine and provider here is ready at once, so a
/// pending future means a test is waiting on something it should not.
pub(crate) fn drive<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("future waited"),
    }
}
