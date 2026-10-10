//! JMAP push: EventSource (RFC 8620 §7.3) and the WebSocket transport
//! (RFC 8887). Frames come from whatever streaming transport the host has;
//! this module only decodes and encodes them, so it opens no socket. A push
//! names new state strings, never mail: the caller compares them with its
//! saved state and runs a sync step.

use std::collections::BTreeMap;
use std::io::Cursor;

use futures_util::{Stream, StreamExt, TryStreamExt, future};
use mailune_protocol::{Http, HttpRequest, Method};
use serde::Deserialize;
use serde_json::{Value, json};
use sse_stream::{Sse, SseByteStream};

use crate::wire::{decode, json_error};
use crate::{Error, JmapClient};

/// RFC 8887 capability in the session.
pub(crate) const WEBSOCKET: &str = "urn:ietf:params:jmap:websocket";

/// The WebSocket capability the session offers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocket {
    /// `wss://` URL, opened with the `jmap` subprotocol.
    pub url: String,
    /// The server sends `StateChange` frames after `WebSocketPushEnable`.
    #[serde(default)]
    pub supports_push: bool,
}

/// New state strings, per account and data type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChange {
    /// Account id, then data type (`Email`, `Mailbox`, …), then its new state.
    pub changed: BTreeMap<String, BTreeMap<String, String>>,
    /// RFC 8887 token to resume pushes after a reconnect. EventSource uses
    /// the event id instead, so it is `None` there.
    #[serde(default)]
    pub push_state: Option<String>,
}

impl StateChange {
    /// The new state of `data_type` in `account`, if the push names it.
    pub fn state(&self, account: &str, data_type: &str) -> Option<&str> {
        self.changed
            .get(account)
            .and_then(|types| types.get(data_type))
            .map(String::as_str)
    }

    /// `saved` is behind the server for `data_type` in `account`.
    pub fn is_newer(&self, account: &str, data_type: &str, saved: &str) -> bool {
        self.state(account, data_type)
            .is_some_and(|state| state != saved)
    }
}

/// One EventSource event the client acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushEvent {
    /// Some state changed.
    State(StateChange),
    /// Keep-alive. The server pings every `interval` seconds, so a longer
    /// silence means the stream is dead.
    Ping {
        /// Seconds between pings.
        interval: u32,
    },
}

/// One message the server sends on the JMAP WebSocket.
#[derive(Debug, Clone, PartialEq)]
pub enum WsMessage {
    /// A push.
    State(StateChange),
    /// The answer to a request sent on the socket.
    Response {
        /// The `id` the request carried, if any.
        request_id: Option<String>,
        /// Method name and arguments, in call order.
        responses: Vec<(String, Value)>,
    },
    /// The server could not process a request.
    RequestError {
        /// The `id` the request carried, if any.
        request_id: Option<String>,
        /// The RFC 7807 problem `type`.
        kind: String,
    },
}

impl<H: Http> JmapClient<'_, H> {
    /// The `GET` that opens the EventSource stream. `types` limits the data
    /// types (`None` is every type); `close_after_state` asks the server to
    /// end the response after one push, for hosts that cannot hold a stream
    /// open. The host's streaming transport sends it and feeds the body to
    /// [`event_source`].
    ///
    /// # Errors
    ///
    /// [`Error::NoSession`] before the session is loaded, [`Error::NoPush`]
    /// when the session has no `eventSourceUrl`.
    pub fn event_source_request(
        &self,
        types: Option<&[&str]>,
        close_after_state: bool,
        ping_seconds: u32,
    ) -> Result<HttpRequest, Error> {
        let template = self
            .session()?
            .event_source_url
            .as_deref()
            .ok_or(Error::NoPush)?;
        let url = expand(
            template,
            &[
                (
                    "types",
                    types.map_or_else(|| "*".into(), |types| types.join(",")),
                ),
                (
                    "closeafter",
                    if close_after_state { "state" } else { "no" }.into(),
                ),
                ("ping", ping_seconds.to_string()),
            ],
        );
        Ok(self
            .authorize(HttpRequest::new(Method::Get, url))
            .header("Accept", "text/event-stream"))
    }
}

/// Typed events from an EventSource body, in chunks as they arrive. Chunks
/// may split an event anywhere. Events of other types are skipped, since
/// RFC 8620 lets a server add them.
pub fn event_source<S, E>(chunks: S) -> impl Stream<Item = Result<PushEvent, Error>>
where
    S: Stream<Item = Result<Vec<u8>, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    SseByteStream::new(chunks.map_ok(Cursor::new))
        .filter_map(|block| future::ready(push_event(block).transpose()))
}

fn push_event(block: Result<Sse, sse_stream::Error>) -> Result<Option<PushEvent>, Error> {
    let block = block.map_err(|err| Error::Stream(err.to_string()))?;
    let Some(data) = block.data.as_deref() else {
        return Ok(None);
    };
    match block.event.as_deref() {
        Some("state") => state_change(serde_json::from_str(data).map_err(json_error)?)
            .map(|change| Some(PushEvent::State(change))),
        Some("ping") => {
            #[derive(Deserialize)]
            struct Ping {
                interval: u32,
            }
            let ping: Ping = serde_json::from_str(data).map_err(json_error)?;
            Ok(Some(PushEvent::Ping {
                interval: ping.interval,
            }))
        }
        _ => Ok(None),
    }
}

/// Decodes one text message from the JMAP WebSocket.
///
/// # Errors
///
/// [`Error::Json`] for a message that is not JSON or has an unknown
/// `@type`; [`Error::Method`] when a response holds a method error.
pub fn ws_message(text: &str) -> Result<WsMessage, Error> {
    let value: Value = serde_json::from_str(text).map_err(json_error)?;
    let request_id = value
        .get("requestId")
        .and_then(Value::as_str)
        .map(str::to_string);
    match value.get("@type").and_then(Value::as_str) {
        Some("StateChange") => state_change(value).map(WsMessage::State),
        Some("Response") => Ok(WsMessage::Response {
            request_id,
            responses: decode(text.as_bytes())?,
        }),
        Some("RequestError") => Ok(WsMessage::RequestError {
            request_id,
            kind: value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        }),
        _ => Err(Error::Json("unknown WebSocket message type".into())),
    }
}

/// `WebSocketPushEnable` for `data_types` (`None` is every type). Passing
/// the last `push_state` makes the server send what was missed.
pub fn push_enable(data_types: Option<&[&str]>, push_state: Option<&str>) -> String {
    let mut message = json!({ "@type": "WebSocketPushEnable", "dataTypes": data_types });
    if let Some(state) = push_state {
        message["pushState"] = json!(state);
    }
    message.to_string()
}

/// `WebSocketPushDisable`.
pub fn push_disable() -> String {
    json!({ "@type": "WebSocketPushDisable" }).to_string()
}

fn state_change(value: Value) -> Result<StateChange, Error> {
    if value.get("@type").and_then(Value::as_str) != Some("StateChange") {
        return Err(Error::Json("push is not a StateChange".into()));
    }
    StateChange::deserialize(value).map_err(json_error)
}

/// RFC 6570 level 1: each `{name}` becomes its value with everything but
/// unreserved characters percent-encoded. Three fixed variables do not
/// warrant a template crate.
fn expand(template: &str, variables: &[(&str, String)]) -> String {
    variables
        .iter()
        .fold(template.to_string(), |url, (name, value)| {
            url.replace(&format!("{{{name}}}"), &percent_encode(value))
        })
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures_util::{StreamExt, stream};
    use mailune_protocol::Secret;
    use mailune_testkit::{ScriptedHttp, poll_now};
    use serde_json::Value;

    use super::{PushEvent, WsMessage, event_source, push_disable, push_enable, ws_message};
    use crate::{Error, JmapClient};

    const SESSION: &str = include_str!("../fixtures/session.json");

    fn events(chunks: &[&str]) -> Vec<Result<PushEvent, Error>> {
        let chunks: Vec<Result<Vec<u8>, Infallible>> = chunks
            .iter()
            .map(|chunk| Ok(chunk.as_bytes().to_vec()))
            .collect();
        poll_now(event_source(stream::iter(chunks)).collect()).unwrap()
    }

    #[test]
    fn the_session_names_both_push_transports() {
        let http = ScriptedHttp::new().json(SESSION);
        let mut client = JmapClient::new(&http, Secret::new("tok-9"));
        assert!(matches!(
            client.event_source_request(None, false, 300),
            Err(Error::NoSession)
        ));
        let session = poll_now(client.load_session("https://x/.well-known/jmap"))
            .unwrap()
            .unwrap();
        let websocket = session.websocket.unwrap();
        assert_eq!(websocket.url, "wss://jmap.example.com/ws/");
        assert!(websocket.supports_push);

        let request = client
            .event_source_request(Some(&["Email", "Mailbox"]), false, 300)
            .unwrap();
        assert_eq!(
            request.url,
            "https://jmap.example.com/events/?types=Email%2CMailbox&closeafter=no&ping=300"
        );
        assert_eq!(request.header_value("accept"), Some("text/event-stream"));
        assert_eq!(request.header_value("authorization"), Some("Bearer tok-9"));
        let every = client.event_source_request(None, true, 60).unwrap();
        assert!(every.url.contains("types=%2A&closeafter=state&ping=60"));
    }

    #[test]
    fn eventsource_frames_split_across_chunks_become_typed_state_changes() {
        let got = events(&[
            ": hello\r\nevent: ping\r\ndata: {\"interval\":300}\r\n\r\nev",
            "ent: state\nid: s-7\ndata: {\"@type\":\"StateChange\",\n",
            "data: \"changed\":{\"acc-1\":{\"Email\":\"e-2\",\"Mailbox\":\"m-1\"}}}\n\n",
            "event: future\ndata: {}\n\n",
        ]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].as_ref().unwrap(), &PushEvent::Ping { interval: 300 });
        let Ok(PushEvent::State(change)) = &got[1] else {
            panic!("expected a state change, got {got:?}");
        };
        assert_eq!(change.state("acc-1", "Email"), Some("e-2"));
        assert!(change.is_newer("acc-1", "Email", "e-1"));
        assert!(!change.is_newer("acc-1", "Email", "e-2"));
        assert!(!change.is_newer("acc-1", "Thread", "t-1"));
        assert_eq!(change.push_state, None);
    }

    #[test]
    fn a_state_event_that_is_not_a_state_change_is_an_error() {
        let got = events(&["event: state\ndata: {\"@type\":\"Other\",\"changed\":{}}\n\n"]);
        assert!(matches!(got[..], [Err(Error::Json(_))]));
        let got = events(&["event: state\ndata: not json\n\n"]);
        assert!(matches!(got[..], [Err(Error::Json(_))]));
    }

    #[test]
    fn websocket_frames_follow_rfc_8887() {
        let push = ws_message(
            r#"{"@type":"StateChange","changed":{"acc-1":{"Email":"e-3"}},"pushState":"p-1"}"#,
        )
        .unwrap();
        let WsMessage::State(change) = push else {
            panic!("expected a state change");
        };
        assert!(change.is_newer("acc-1", "Email", "e-2"));
        assert_eq!(change.push_state.as_deref(), Some("p-1"));

        let response = ws_message(
            r#"{"@type":"Response","requestId":"r1","methodResponses":[["Email/get",{"list":[]},"c0"]],"sessionState":"s"}"#,
        )
        .unwrap();
        let WsMessage::Response {
            request_id,
            responses,
        } = response
        else {
            panic!("expected a response");
        };
        assert_eq!(request_id.as_deref(), Some("r1"));
        assert_eq!(responses[0].0, "Email/get");

        let refused = ws_message(
            r#"{"@type":"RequestError","requestId":"r2","type":"urn:ietf:params:jmap:error:notJSON","status":400}"#,
        )
        .unwrap();
        assert_eq!(
            refused,
            WsMessage::RequestError {
                request_id: Some("r2".into()),
                kind: "urn:ietf:params:jmap:error:notJSON".into(),
            }
        );
        assert!(matches!(
            ws_message(r#"{"@type":"Request"}"#),
            Err(Error::Json(_))
        ));

        let enable: Value =
            serde_json::from_str(&push_enable(Some(&["Email"]), Some("p-1"))).unwrap();
        assert_eq!(enable["@type"], "WebSocketPushEnable");
        assert_eq!(enable["dataTypes"][0], "Email");
        assert_eq!(enable["pushState"], "p-1");
        let every: Value = serde_json::from_str(&push_enable(None, None)).unwrap();
        assert!(every["dataTypes"].is_null());
        assert!(every.get("pushState").is_none());
        assert_eq!(push_disable(), r#"{"@type":"WebSocketPushDisable"}"#);
    }
}
