//! A transport that answers from recorded bodies, so the web shell can run
//! a JMAP call against fixture data before it has a fetch transport.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use mailune_protocol::{Error, Http, HttpRequest, HttpResponse};

/// Answers each request with the next recorded body and status 200.
pub(crate) struct Replay {
    bodies: Mutex<VecDeque<String>>,
}

impl Replay {
    pub(crate) fn new(bodies: Vec<String>) -> Self {
        Self {
            bodies: Mutex::new(bodies.into()),
        }
    }
}

impl Http for Replay {
    async fn send(&self, _request: HttpRequest) -> Result<HttpResponse, Error> {
        self.bodies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .map(|body| HttpResponse::new(200, body))
            .ok_or_else(|| Error::host("http.send", "no recorded reply left"))
    }
}

/// Runs a future that never waits: every reply is already recorded.
pub(crate) fn run<T>(future: impl Future<Output = T>) -> Result<T, String> {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => Ok(value),
        Poll::Pending => Err("the call waited on something other than a recorded reply".into()),
    }
}
