//! [`ScriptedHttp`]: an [`Http`] fake that answers from a queue of canned
//! responses and records every request, so provider adapters are tested
//! without a socket.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

use mailune_protocol::{Error, Http, HttpRequest, HttpResponse};

#[derive(Default)]
struct Script {
    replies: VecDeque<HttpResponse>,
    seen: Vec<HttpRequest>,
}

/// Answers requests in order from a script.
#[derive(Default)]
pub struct ScriptedHttp {
    script: Mutex<Script>,
}

impl std::fmt::Debug for ScriptedHttp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ScriptedHttp")
    }
}

impl ScriptedHttp {
    /// An empty script. Every request fails until replies are queued.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `response` for the next unanswered request.
    #[must_use]
    pub fn reply(self, response: HttpResponse) -> Self {
        self.lock().replies.push_back(response);
        self
    }

    /// Queues a 200 response with a JSON body.
    #[must_use]
    pub fn json(self, body: impl Into<Vec<u8>>) -> Self {
        self.reply(HttpResponse::new(200, body).header("Content-Type", "application/json"))
    }

    /// Requests received so far, oldest first.
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.lock().seen.clone()
    }

    /// Replies still queued.
    pub fn remaining(&self) -> usize {
        self.lock().replies.len()
    }

    fn lock(&self) -> MutexGuard<'_, Script> {
        // A poisoned script is still readable; a failed test must not hang the next.
        self.script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Http for ScriptedHttp {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, Error> {
        let mut script = self.lock();
        script.seen.push(request);
        script
            .replies
            .pop_front()
            .ok_or_else(|| Error::host("http.send", "no scripted reply left"))
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Http, HttpRequest, HttpResponse, Method};

    use super::ScriptedHttp;
    use crate::poll_now;

    #[test]
    fn replies_come_back_in_order_and_requests_are_recorded() {
        let http = ScriptedHttp::new()
            .json(r#"{"a":1}"#)
            .reply(HttpResponse::new(404, ""));
        let first = poll_now(http.send(HttpRequest::new(Method::Get, "https://x/1"))).unwrap();
        let second = poll_now(http.send(HttpRequest::new(Method::Post, "https://x/2"))).unwrap();
        assert_eq!((first.unwrap().status, second.unwrap().status), (200, 404));
        let third = poll_now(http.send(HttpRequest::new(Method::Get, "https://x/3"))).unwrap();
        assert!(third.is_err());
        let urls: Vec<String> = http.requests().into_iter().map(|r| r.url).collect();
        assert_eq!(urls, ["https://x/1", "https://x/2", "https://x/3"]);
    }
}
