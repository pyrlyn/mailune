//! JSON-RPC methods. Each call is a typed [`Method`] forwarded to a [`Handler`].
//!
//! The handler returns protocol [`Event`]s. This crate does not open a socket
//! or print: the caller supplies the request text and takes the response text.
//! Schemas describe the params; the JSON-RPC envelope is not a second schema.

use mailune_protocol::{Address, Event, ThreadId};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Failure from a handler. Parse failures stay inside the JSON-RPC response
/// so a bad client does not need a second error channel.
///
/// The text is safe to return to a client: it does not carry a token.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The handler declined the call.
    #[error("the handler refused the call")]
    Refused,
}

/// Params for `open_thread`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenThreadParams {
    /// Conversation to open.
    pub thread: ThreadId,
}

/// Params for `search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Query string, including tokens such as `from:`.
    pub query: String,
}

/// Params for `save_draft`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SaveDraftParams {
    /// Recipients so far.
    pub to: Vec<Address>,
    /// Subject line.
    pub subject: String,
    /// Plain body.
    pub body: String,
}

/// Params for `set_language`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SetLanguageParams {
    /// Language code, or empty for English.
    pub code: String,
}

/// One method the surface can call. `refresh` has no params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    /// Reload the visible mailbox.
    Refresh,
    /// Open one conversation.
    OpenThread(OpenThreadParams),
    /// Run a search query.
    Search(SearchParams),
    /// Store a draft without sending.
    SaveDraft(SaveDraftParams),
    /// Switch UI language.
    SetLanguage(SetLanguageParams),
}

/// Receives a typed method and returns the events the UI should apply.
pub trait Handler {
    /// Apply `method`.
    ///
    /// # Errors
    ///
    /// [`Error::Refused`] when the handler will not run the call. The message
    /// must stay free of tokens and message bodies.
    fn handle(&mut self, method: Method) -> Result<Vec<Event>, Error>;
}

/// Request id. A notification (no id) is not accepted: this surface always
/// answers with events, so there is nothing to omit the id for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric id.
    Number(i64),
    /// String id.
    Text(String),
}

#[derive(Deserialize)]
struct Incoming {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
    id: RequestId,
}

/// Runs one JSON-RPC request and returns the response JSON.
///
/// A parse or method error is still a JSON-RPC response. The handler is not
/// called in that case.
pub fn dispatch(handler: &mut impl Handler, request: &str) -> String {
    let incoming = match serde_json::from_str::<Incoming>(request) {
        Ok(incoming) => incoming,
        Err(err) if err.is_data() || err.is_syntax() => {
            return encode(&failure(None, PARSE_ERROR, "request is not JSON-RPC"));
        }
        Err(_) => {
            return encode(&failure(
                None,
                INVALID_REQUEST,
                "request is not a JSON-RPC call",
            ));
        }
    };
    if incoming.jsonrpc != "2.0" {
        return encode(&failure(
            Some(incoming.id),
            INVALID_REQUEST,
            "jsonrpc must be 2.0",
        ));
    }
    let method = match method_from(&incoming.method, incoming.params) {
        Ok(method) => method,
        Err(MethodError::Unknown) => {
            return encode(&failure(
                Some(incoming.id),
                METHOD_NOT_FOUND,
                "method is not known",
            ));
        }
        Err(MethodError::Params) => {
            return encode(&failure(
                Some(incoming.id),
                INVALID_PARAMS,
                "params do not match the method",
            ));
        }
    };
    match handler.handle(method) {
        Ok(events) => encode(&success(incoming.id, &events)),
        Err(err) => encode(&failure(Some(incoming.id), HANDLER, &err.to_string())),
    }
}

enum MethodError {
    Unknown,
    Params,
}

fn method_from(name: &str, params: serde_json::Value) -> Result<Method, MethodError> {
    match name {
        "refresh" => {
            if params.is_null() || params == serde_json::json!({}) {
                Ok(Method::Refresh)
            } else {
                Err(MethodError::Params)
            }
        }
        "open_thread" => parse(params).map(Method::OpenThread),
        "search" => parse(params).map(Method::Search),
        "save_draft" => parse(params).map(Method::SaveDraft),
        "set_language" => parse(params).map(Method::SetLanguage),
        _ => Err(MethodError::Unknown),
    }
}

fn parse<T: DeserializeOwned>(params: serde_json::Value) -> Result<T, MethodError> {
    serde_json::from_value(params).map_err(|_| MethodError::Params)
}

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const HANDLER: i64 = -32000;

#[derive(Serialize)]
struct RpcErrorBody<'a> {
    code: i64,
    message: &'a str,
}

#[derive(Serialize)]
struct Success<'a> {
    jsonrpc: &'a str,
    id: RequestId,
    result: &'a Vec<Event>,
}

#[derive(Serialize)]
struct Failure<'a> {
    jsonrpc: &'a str,
    id: Option<RequestId>,
    error: RpcErrorBody<'a>,
}

fn success(id: RequestId, events: &Vec<Event>) -> Success<'_> {
    Success {
        jsonrpc: "2.0",
        id,
        result: events,
    }
}

fn failure<'a>(id: Option<RequestId>, code: i64, message: &'a str) -> Failure<'a> {
    Failure {
        jsonrpc: "2.0",
        id,
        error: RpcErrorBody { code, message },
    }
}

fn encode(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"response could not be encoded"}}"#
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use mailune_protocol::{Address, Event, ThreadId};
    use schemars::schema_for;

    use super::{
        Error, Handler, Method, OpenThreadParams, SaveDraftParams, SearchParams, SetLanguageParams,
        dispatch,
    };

    struct Rec {
        last: Option<Method>,
        refuse: bool,
    }

    impl Handler for Rec {
        fn handle(&mut self, method: Method) -> Result<Vec<Event>, Error> {
            self.last = Some(method);
            if self.refuse {
                Err(Error::Refused)
            } else {
                Ok(vec![Event::Notice {
                    message: "ok".into(),
                }])
            }
        }
    }

    fn call(request: &str) -> (serde_json::Value, Rec) {
        let mut handler = Rec {
            last: None,
            refuse: false,
        };
        let raw = dispatch(&mut handler, request);
        let value = serde_json::from_str(&raw).unwrap();
        (value, handler)
    }

    #[test]
    fn open_thread_forwards_params_and_returns_events() {
        let (value, handler) =
            call(r#"{"jsonrpc":"2.0","id":7,"method":"open_thread","params":{"thread":"t1"}}"#);
        assert_eq!(
            handler.last,
            Some(Method::OpenThread(OpenThreadParams {
                thread: ThreadId::new("t1"),
            }))
        );
        assert_eq!(value["id"], 7);
        assert_eq!(value["result"][0]["notice"]["message"], "ok");
        assert!(value.get("error").is_none());
    }

    #[test]
    fn save_draft_keeps_the_typed_params() {
        let (value, handler) = call(
            r#"{"jsonrpc":"2.0","id":"a","method":"save_draft","params":{"to":[{"name":null,"email":"ada@example.com"}],"subject":"Hi","body":"Hello"}}"#,
        );
        assert_eq!(
            handler.last,
            Some(Method::SaveDraft(SaveDraftParams {
                to: vec![Address {
                    name: None,
                    email: "ada@example.com".into(),
                }],
                subject: "Hi".into(),
                body: "Hello".into(),
            }))
        );
        assert_eq!(value["id"], "a");
    }

    #[test]
    fn refresh_accepts_empty_params() {
        let (_, handler) = call(r#"{"jsonrpc":"2.0","id":1,"method":"refresh"}"#);
        assert_eq!(handler.last, Some(Method::Refresh));
    }

    #[test]
    fn unknown_method_is_not_forwarded() {
        let (value, handler) = call(r#"{"jsonrpc":"2.0","id":1,"method":"drop_table"}"#);
        assert!(handler.last.is_none());
        assert_eq!(value["error"]["code"], -32601);
    }

    #[test]
    fn bad_params_are_rejected() {
        let (value, handler) =
            call(r#"{"jsonrpc":"2.0","id":1,"method":"search","params":{"query":1}}"#);
        assert!(handler.last.is_none());
        assert_eq!(value["error"]["code"], -32602);
        let _ = SearchParams {
            query: String::new(),
        };
    }

    #[test]
    fn a_refused_call_is_an_error_object() {
        let mut handler = Rec {
            last: None,
            refuse: true,
        };
        let raw = dispatch(
            &mut handler,
            r#"{"jsonrpc":"2.0","id":1,"method":"set_language","params":{"code":"fr"}}"#,
        );
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["error"]["code"], -32000);
        assert!(handler.last.is_some());
        let _ = SetLanguageParams { code: "fr".into() };
    }

    #[test]
    fn broken_json_does_not_call_the_handler() {
        let (value, handler) = call("not json");
        assert!(handler.last.is_none());
        assert_eq!(value["error"]["code"], -32700);
        assert!(value["id"].is_null());
    }

    #[test]
    fn param_schemas_name_their_fields() {
        let open = schema_for!(OpenThreadParams);
        let search = schema_for!(SearchParams);
        let draft = schema_for!(SaveDraftParams);
        let language = schema_for!(SetLanguageParams);
        let text = serde_json::to_string(&open).unwrap();
        assert!(text.contains("thread"), "{text}");
        assert!(serde_json::to_string(&search).unwrap().contains("query"));
        assert!(serde_json::to_string(&draft).unwrap().contains("subject"));
        assert!(serde_json::to_string(&language).unwrap().contains("code"));
    }
}
