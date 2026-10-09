//! HTTP transport for the JSON providers (JMAP, Gmail API, Microsoft Graph).
//!
//! Adapters build a request and hand it to an [`Http`] implementation the
//! host or `mailune-app` injects. Tests inject a scripted one, so no adapter
//! opens a socket. The `Authorization` header is redacted from `Debug`.

use std::fmt;
use std::future::Future;

use crate::Error;

/// Request method. Only the verbs the mail providers use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
}

/// One request.
#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// Verb.
    pub method: Method,
    /// Absolute URL.
    pub url: String,
    /// Header names and values in order.
    pub headers: Vec<(String, String)>,
    /// Body bytes; empty for `GET`.
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// A request with no headers and no body.
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Adds a header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets a JSON body and its content type.
    #[must_use]
    pub fn json(self, body: impl Into<Vec<u8>>) -> Self {
        let mut request = self.header("Content-Type", "application/json");
        request.body = body.into();
        request
    }

    /// The first value of header `name`, compared without case.
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headers: Vec<(&str, &str)> = self
            .headers
            .iter()
            .map(|(name, value)| {
                if name.eq_ignore_ascii_case("authorization") {
                    (name.as_str(), "redacted")
                } else {
                    (name.as_str(), value.as_str())
                }
            })
            .collect();
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &headers)
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// One response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Header names and values in order.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// A response with a status and a body and no headers.
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// Adds a header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// The first value of header `name`, compared without case.
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Sends HTTP requests. TLS, proxies and redirects belong to the
/// implementation.
pub trait Http: Send + Sync {
    /// Sends `request` and returns the whole response.
    ///
    /// A non-2xx status is a response, not an error.
    ///
    /// # Errors
    ///
    /// [`Error::Host`] when no response arrived.
    fn send(
        &self,
        request: HttpRequest,
    ) -> impl Future<Output = Result<HttpResponse, Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::{HttpRequest, Method};

    #[test]
    fn debug_hides_the_authorization_header() {
        let request = HttpRequest::new(Method::Get, "https://example.com/")
            .header("Authorization", "Bearer secret-token")
            .header("Accept", "application/json");
        let text = format!("{request:?}");
        assert!(!text.contains("secret-token"));
        assert!(text.contains("application/json"));
        assert_eq!(
            request.header_value("authorization"),
            Some("Bearer secret-token")
        );
    }
}
