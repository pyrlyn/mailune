//! The WebSocket endpoint: token check, then one JSON-RPC call per text frame.
//!
//! Each frame is handed to [`mailune_rpc::dispatch`] as it is, and its answer
//! goes back as one text frame. This module owns transport and auth only;
//! method parsing and errors are `mailune-rpc`'s.

use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mailune_rpc::Handler;
use rand::RngCore;
use rand::rngs::OsRng;

/// Where the endpoint is mounted.
pub const RPC_PATH: &str = "/rpc";
/// The subprotocol the server speaks, echoed back on upgrade.
const PROTOCOL: &str = "mailune-rpc";
/// Prefix of the subprotocol that carries the token. Browsers cannot set an
/// `Authorization` header on a WebSocket, but they can list subprotocols.
const TOKEN_PROTOCOL_PREFIX: &str = "mailune.token.";
/// Random bytes in a token: 256 bits, beyond guessing.
const TOKEN_BYTES: usize = 32;
/// The largest frame accepted. Requests are small JSON; a larger frame is a
/// client bug or an attempt to exhaust memory.
const MAX_MESSAGE: usize = 1024 * 1024;

/// The bearer token a client must present. Its `Debug` never shows it.
pub struct Token(String);

impl Token {
    /// A fresh random token, base64url without padding so it is also a valid
    /// subprotocol name.
    pub fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_BYTES];
        OsRng.fill_bytes(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// The token text, to show the operator once.
    pub fn reveal(&self) -> &str {
        &self.0
    }

    /// Compares without stopping at the first differing byte, so the time
    /// taken does not reveal how much of a guess was right.
    fn matches(&self, presented: &str) -> bool {
        let (own, other) = (self.0.as_bytes(), presented.as_bytes());
        own.len() == other.len()
            && own
                .iter()
                .zip(other)
                .fold(0u8, |diff, (a, b)| diff | (a ^ b))
                == 0
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(redacted)")
    }
}

struct Shared<H> {
    token: Token,
    handler: Mutex<H>,
}

/// The router: one route, [`RPC_PATH`].
pub fn router<H: Handler + Send + 'static>(token: Token, handler: H) -> Router {
    let shared = Arc::new(Shared {
        token,
        handler: Mutex::new(handler),
    });
    Router::new()
        .route(RPC_PATH, get(upgrade::<H>))
        .with_state(shared)
}

/// Serves [`router`] on `listener` until the process stops.
pub async fn serve<H: Handler + Send + 'static>(
    listener: tokio::net::TcpListener,
    token: Token,
    handler: H,
) -> std::io::Result<()> {
    axum::serve(listener, router(token, handler)).await
}

async fn upgrade<H: Handler + Send + 'static>(
    State(shared): State<Arc<Shared<H>>>,
    headers: HeaderMap,
    socket: WebSocketUpgrade,
) -> Response {
    if !presented_tokens(&headers).any(|token| shared.token.matches(token)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    socket
        .protocols([PROTOCOL])
        .max_message_size(MAX_MESSAGE)
        .on_upgrade(move |socket| session(socket, shared))
}

/// Tokens from `Authorization: Bearer` and from `mailune.token.*` subprotocols.
fn presented_tokens(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    let bearer = headers
        .get_all(header::AUTHORIZATION)
        .into_iter()
        .filter_map(|value| value.to_str().ok()?.strip_prefix("Bearer "));
    let protocols = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|list| list.split(','))
        .filter_map(|protocol| protocol.trim().strip_prefix(TOKEN_PROTOCOL_PREFIX));
    bearer.chain(protocols)
}

async fn session<H: Handler>(mut socket: WebSocket, shared: Arc<Shared<H>>) {
    while let Some(Ok(message)) = socket.recv().await {
        let request = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            // Pings are answered by axum; binary frames are not JSON-RPC.
            _ => continue,
        };
        let reply = {
            // A handler that panicked mid-call leaves its state as it was;
            // refusing every later call would turn one bug into an outage.
            let mut handler = shared
                .handler
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            mailune_rpc::dispatch(&mut *handler, request.as_str())
        };
        if socket.send(Message::Text(reply.into())).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use mailune_protocol::Event;
    use mailune_rpc::{Error, Handler, Method};
    use serde_json::Value;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::HeaderValue;
    use tokio_tungstenite::tungstenite::{Error as WsError, Message};

    use super::{RPC_PATH, Token, serve};

    const TOKEN: &str = "test-token-abc";

    /// Answers `search` with a notice naming the query; refuses the rest.
    struct Echo;

    impl Handler for Echo {
        fn handle(&mut self, method: Method) -> Result<Vec<Event>, Error> {
            match method {
                Method::Search(params) => Ok(vec![Event::Notice {
                    message: format!("searched {}", params.query),
                }]),
                _ => Err(Error::Refused),
            }
        }
    }

    /// Starts the server on an ephemeral loopback port and returns its URL.
    async fn start() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        // A loopback test server: TLS is not what these tests check.
        let url = format!("ws://{}{RPC_PATH}", listener.local_addr().unwrap()); // nosemgrep
        tokio::spawn(serve(listener, Token(TOKEN.into()), Echo));
        url
    }

    async fn call(
        url: &str,
        header: (&'static str, String),
        request: &str,
    ) -> Result<Value, WsError> {
        let mut req = url.into_client_request()?;
        req.headers_mut()
            .insert(header.0, HeaderValue::from_str(&header.1).unwrap());
        let (mut socket, _) = tokio_tungstenite::connect_async(req).await?;
        socket.send(Message::text(request)).await?;
        let reply = socket.next().await.unwrap()?;
        Ok(serde_json::from_str(reply.to_text()?).unwrap())
    }

    const SEARCH: &str =
        r#"{"jsonrpc":"2.0","id":1,"method":"search","params":{"query":"from:ada"}}"#;

    #[tokio::test]
    async fn a_search_answers_over_the_socket_with_a_bearer_token() {
        let url = start().await;
        let reply = call(&url, ("authorization", format!("Bearer {TOKEN}")), SEARCH)
            .await
            .unwrap();
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"][0]["notice"]["message"], "searched from:ada");
    }

    #[tokio::test]
    async fn a_browser_can_send_the_token_as_a_subprotocol() {
        let url = start().await;
        let protocols = format!("mailune-rpc, mailune.token.{TOKEN}");
        let reply = call(&url, ("sec-websocket-protocol", protocols), SEARCH)
            .await
            .unwrap();
        assert_eq!(reply["result"][0]["notice"]["message"], "searched from:ada");
    }

    #[tokio::test]
    async fn a_missing_or_wrong_token_is_refused_before_the_upgrade() {
        let url = start().await;
        for header in [
            ("x-nothing", String::new()),
            ("authorization", "Bearer test-token-abd".to_string()),
            ("authorization", format!("Basic {TOKEN}")),
        ] {
            match call(&url, header, SEARCH).await {
                Err(WsError::Http(response)) => assert_eq!(response.status(), 401),
                other => panic!("expected 401, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_bad_request_is_a_json_rpc_error_not_a_dropped_socket() {
        let url = start().await;
        let reply = call(
            &url,
            ("authorization", format!("Bearer {TOKEN}")),
            "not json",
        )
        .await
        .unwrap();
        assert_eq!(reply["error"]["code"], -32700);
    }

    #[test]
    fn a_token_is_random_and_redacted() {
        let (a, b) = (Token::generate(), Token::generate());
        assert_ne!(a.reveal(), b.reveal());
        assert_eq!(format!("{a:?}"), "Token(redacted)");
        assert!(a.matches(a.reveal()));
        assert!(!a.matches(b.reveal()));
        assert!(!a.matches(""));
    }
}
