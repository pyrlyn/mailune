//! The webhook router. Each registration owns an unguessable channel id in
//! its webhook URL; a screened notice on that URL wakes the device behind
//! it with an empty push. The device then syncs with its own credentials.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;

use crate::screen::{Provider, Refusal, screen};

/// Graph's validation token is short; anything longer is not one.
const MAX_VALIDATION: usize = 1024;

/// An opaque id for a device's push registration. It is not a provider
/// credential and says nothing about mail.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceHandle(pub String);

/// Maps a webhook channel to the device it wakes.
pub trait Registry: Send + Sync + 'static {
    /// The device for `channel`, if one is registered.
    fn device(&self, provider: Provider, channel: &str) -> Option<DeviceHandle>;
}

/// Delivers an empty wake. Real APNs and FCM senders implement this; tests
/// use a recorder.
pub trait Notifier: Send + Sync + 'static {
    /// Wakes `device`. The push carries nothing else.
    fn wake(&self, device: &DeviceHandle);
}

/// An in-memory registry: channel to device handle, nothing more.
#[derive(Debug, Default)]
pub struct MemoryRegistry {
    channels: Mutex<HashMap<(Provider, String), DeviceHandle>>,
}

impl MemoryRegistry {
    /// Routes `channel` for `provider` to `device`.
    pub fn register(&self, provider: Provider, channel: &str, device: DeviceHandle) {
        // A poisoned map still holds valid entries; a panic elsewhere must
        // not stop wakes.
        self.channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((provider, channel.to_string()), device);
    }
}

impl Registry for MemoryRegistry {
    fn device(&self, provider: Provider, channel: &str) -> Option<DeviceHandle> {
        self.channels
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&(provider, channel.to_string()))
            .cloned()
    }
}

struct Relay<R, N> {
    registry: R,
    notifier: N,
}

/// The relay's routes: `POST /hook/{gmail|graph}/{channel}`.
pub fn router<R: Registry, N: Notifier>(registry: R, notifier: N) -> Router {
    Router::new()
        .route("/hook/{provider}/{channel}", post(hook::<R, N>))
        .with_state(Arc::new(Relay { registry, notifier }))
}

async fn hook<R: Registry, N: Notifier>(
    State(relay): State<Arc<Relay<R, N>>>,
    Path((provider, channel)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let Some(provider) = Provider::from_path(&provider) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(device) = relay.registry.device(provider, &channel) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Graph proves a subscription URL by asking it to echo a token.
    if let (Provider::Graph, Some(token)) = (provider, query.get("validationToken")) {
        return validation(token);
    }
    match screen(provider, &body) {
        Ok(()) => {
            relay.notifier.wake(&device);
            StatusCode::ACCEPTED.into_response()
        }
        Err(Refusal::TooLarge) => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        // The reason stays out of the response so nothing is echoed back.
        Err(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    }
}

fn validation(token: &str) -> Response {
    let printable = token.chars().all(|c| !c.is_control());
    if token.is_empty() || token.len() > MAX_VALIDATION || !printable {
        return StatusCode::BAD_REQUEST.into_response();
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        token.to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;
    use tower::ServiceExt;

    use super::{DeviceHandle, MemoryRegistry, Notifier, router};
    use crate::screen::Provider;

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<DeviceHandle>>>);

    impl Notifier for Recorder {
        fn wake(&self, device: &DeviceHandle) {
            self.0.lock().unwrap().push(device.clone());
        }
    }

    fn relay() -> (axum::Router, Recorder) {
        let registry = MemoryRegistry::default();
        registry.register(Provider::Gmail, "c-gm", DeviceHandle("dev-1".into()));
        registry.register(Provider::Graph, "c-ms", DeviceHandle("dev-2".into()));
        let recorder = Recorder::default();
        (router(registry, recorder.clone()), recorder)
    }

    async fn post(app: &axum::Router, uri: &str, body: String) -> (StatusCode, String) {
        let request = Request::post(uri).body(Body::from(body)).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn notices_become_empty_wakes() {
        let (app, recorder) = relay();
        let data = STANDARD.encode(r#"{"emailAddress":"me@gmail.com","historyId":"42"}"#);
        let gmail = json!({ "message": { "data": data, "messageId": "1" }, "subscription": "s" });
        let (status, body) = post(&app, "/hook/gmail/c-gm", gmail.to_string()).await;
        assert_eq!((status, body.as_str()), (StatusCode::ACCEPTED, ""));
        let graph = json!({ "value": [{ "subscriptionId": "s", "changeType": "created",
            "resource": "Users/u/Messages/m", "resourceData": { "id": "m" } }] });
        let (status, _) = post(&app, "/hook/graph/c-ms", graph.to_string()).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(
            *recorder.0.lock().unwrap(),
            [DeviceHandle("dev-1".into()), DeviceHandle("dev-2".into())]
        );
    }

    #[tokio::test]
    async fn content_unknown_channels_and_bad_validation_are_refused() {
        let (app, recorder) = relay();
        let rich = json!({ "value": [{ "resourceData": { "subject": "Payroll" } }] });
        let (status, body) = post(&app, "/hook/graph/c-ms", rich.to_string()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!body.contains("Payroll"));
        let (status, _) = post(&app, "/hook/graph/nope", "{}".into()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = post(&app, "/hook/imap/c-ms", "{}".into()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = post(&app, "/hook/gmail/c-gm", " ".repeat(20_000)).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        let (status, _) = post(&app, "/hook/graph/c-ms?validationToken=", String::new()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(recorder.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn graph_validation_echoes_the_token_as_plain_text() {
        let (app, recorder) = relay();
        let request = Request::post("/hook/graph/c-ms?validationToken=Validation%3A%20abc")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-type"],
            "text/plain; charset=utf-8"
        );
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(&bytes[..], b"Validation: abc");
        assert!(recorder.0.lock().unwrap().is_empty());
    }
}
