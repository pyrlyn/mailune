//! Push relay.
//!
//! A Gmail or Graph webhook becomes one account id. A body that carries a
//! token, a subject, or mail text is refused and not stored. The process
//! does not call Apple or Google.

mod client;

pub use client::{Client, Registration};

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use serde_json::Value;

const FORBIDDEN: &[&str] = &[
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "subject",
    "body",
    "bodypreview",
    "snippet",
    "text",
    "content",
];

/// Why a webhook was not turned into a wake.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The body is not a JSON object the relay accepts.
    #[error("webhook JSON is not valid")]
    Json,
    /// The body carries a token, a subject, or mail text.
    #[error("webhook carries mail or a secret")]
    Refused,
    /// No account id was present.
    #[error("webhook has no account")]
    Account,
    /// The in-memory wake list could not be locked.
    #[error("relay state could not be locked")]
    Lock,
    /// A device id was empty.
    #[error("device id is empty")]
    Device,
}

/// An empty wake. The only field is the account to sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wake {
    /// Provider account id, or the mailbox address when that is the id.
    pub account_id: String,
}

/// Account ids the relay has accepted. The raw webhook is never kept.
#[derive(Clone, Default)]
pub struct Relay {
    accounts: Arc<Mutex<Vec<String>>>,
}

impl Relay {
    /// An empty relay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Account ids accepted so far, in arrival order.
    ///
    /// # Errors
    ///
    /// [`Error::Lock`] when the list lock is poisoned.
    pub fn account_ids(&self) -> Result<Vec<String>, Error> {
        self.accounts
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| Error::Lock)
    }

    fn record(&self, account_id: &str) -> Result<(), Error> {
        let mut guard = self.accounts.lock().map_err(|_| Error::Lock)?;
        if !guard.iter().any(|id| id == account_id) {
            guard.push(account_id.to_string());
        }
        Ok(())
    }
}

/// Axum routes `/gmail` and `/graph`. The caller serves them; this crate does not bind.
pub fn router(relay: Relay) -> Router {
    Router::new()
        .route("/gmail", post(gmail))
        .route("/graph", post(graph))
        .with_state(relay)
}

/// Gmail webhook to one account id.
///
/// # Errors
///
/// [`Error::Json`], [`Error::Refused`], or [`Error::Account`].
pub fn gmail_wake(body: &str) -> Result<Wake, Error> {
    wake(body, Provider::Gmail)
}

/// Graph webhook to one account id.
///
/// # Errors
///
/// [`Error::Json`], [`Error::Refused`], or [`Error::Account`].
pub fn graph_wake(body: &str) -> Result<Wake, Error> {
    wake(body, Provider::Graph)
}

#[derive(Clone, Copy)]
enum Provider {
    Gmail,
    Graph,
}

fn wake(body: &str, provider: Provider) -> Result<Wake, Error> {
    let value: Value = serde_json::from_str(body).map_err(|_| Error::Json)?;
    if carries_secret(&value) {
        return Err(Error::Refused);
    }
    let mut ids = Vec::new();
    collect_ids(&value, provider, &mut ids);
    let account_id = ids
        .into_iter()
        .find(|id| usable(id))
        .ok_or(Error::Account)?;
    Ok(Wake { account_id })
}

fn usable(id: &str) -> bool {
    !id.is_empty() && id.len() <= 320 && !id.chars().any(char::is_whitespace)
}

fn carries_secret(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            FORBIDDEN.iter().any(|name| key.eq_ignore_ascii_case(name)) || carries_secret(child)
        }),
        Value::Array(items) => items.iter().any(carries_secret),
        _ => false,
    }
}

fn collect_ids(value: &Value, provider: Provider, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            match provider {
                Provider::Gmail => {
                    push_str(map.get("accountId"), out);
                    push_str(map.get("emailAddress"), out);
                }
                Provider::Graph => {
                    if let Some(resource) = map.get("resource").and_then(Value::as_str)
                        && let Some(id) = account_from_resource(resource)
                    {
                        push_unique(out, id);
                    }
                }
            }
            for child in map.values() {
                collect_ids(child, provider, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_ids(item, provider, out);
            }
        }
        _ => {}
    }
}

fn push_str(value: Option<&Value>, out: &mut Vec<String>) {
    if let Some(id) = value.and_then(Value::as_str) {
        push_unique(out, id.to_string());
    }
}

fn push_unique(out: &mut Vec<String>, id: String) {
    if !out.iter().any(|have| have == &id) {
        out.push(id);
    }
}

fn account_from_resource(resource: &str) -> Option<String> {
    let parts: Vec<&str> = resource
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for (index, part) in parts.iter().enumerate() {
        if part.eq_ignore_ascii_case("users") {
            let id = parts.get(index + 1)?;
            let id = id.trim_matches('\'');
            if id.is_empty() {
                return None;
            }
            return Some(id.to_string());
        }
    }
    None
}

async fn gmail(State(relay): State<Relay>, body: String) -> StatusCode {
    accept(&relay, gmail_wake(&body))
}

async fn graph(State(relay): State<Relay>, body: String) -> StatusCode {
    accept(&relay, graph_wake(&body))
}

fn accept(relay: &Relay, wake: Result<Wake, Error>) -> StatusCode {
    match wake {
        Ok(wake) => match relay.record(&wake.account_id) {
            Ok(()) => StatusCode::NO_CONTENT,
            Err(Error::Lock) => StatusCode::INTERNAL_SERVER_ERROR,
            Err(_) => StatusCode::BAD_REQUEST,
        },
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

#[cfg(test)]
mod tests {
    use super::{Relay, gmail_wake, graph_wake, router};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn a_secret_or_subject_is_refused_and_not_stored() {
        let relay = Relay::new();
        let err = gmail_wake(r#"{"accountId":"acc-1","subject":"Hello","body":"secret mail"}"#)
            .unwrap_err();
        assert!(matches!(err, super::Error::Refused));
        let err =
            graph_wake(r#"{"value":[{"resource":"Users/acc-1/Messages/AAMk","token":"sekret"}]}"#)
                .unwrap_err();
        assert!(matches!(err, super::Error::Refused));
        assert!(relay.account_ids().unwrap().is_empty());
        let rendered = format!("{err:?}");
        assert!(!rendered.contains("sekret"));
        assert!(!rendered.contains("secret mail"));
    }

    #[test]
    fn gmail_and_graph_wakes_keep_only_the_account() {
        let gmail = gmail_wake(r#"{"emailAddress":"me@example.com","historyId":"100"}"#).unwrap();
        assert_eq!(gmail.account_id, "me@example.com");
        let graph = graph_wake(
            r#"{"value":[{"changeType":"created","resource":"Users/acc-1/Messages/AAMk"}]}"#,
        )
        .unwrap();
        assert_eq!(graph.account_id, "acc-1");
    }

    #[tokio::test]
    async fn the_router_records_an_account_and_drops_mail() {
        let relay = Relay::new();
        let app = router(relay.clone());
        let ok = app
            .clone()
            .oneshot(
                Request::post("/gmail")
                    .body(Body::from(r#"{"accountId":"acc-1","historyId":"5"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::NO_CONTENT);
        let denied = app
            .oneshot(
                Request::post("/graph")
                    .body(Body::from(
                        r#"{"value":[{"resource":"Users/acc-1/Messages/AAMk","subject":"Hello"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
        assert_eq!(relay.account_ids().unwrap(), ["acc-1"]);
    }
}
