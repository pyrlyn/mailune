//! WebSocket JSON-RPC server.
//!
//! The methods are the ones `mailune-rpc` already defines. A token is compared
//! before the upgrade and is never written to a log. Passkeys are not part of
//! this server.

use std::env;

use anyhow::Context;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use mailune_protocol::Event;
use mailune_rpc::{Handler, Method};
use serde::Deserialize;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = env::var("MAILUNE_TOKEN").context("MAILUNE_TOKEN is required")?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    eprintln!("listening on {addr}");
    serve(token, listener).await
}

/// Serves `listener` until it closes. An empty token is refused before accept.
///
/// # Errors
///
/// Returns an error when `token` is empty or the server stops with an I/O error.
pub async fn serve(token: String, listener: TcpListener) -> anyhow::Result<()> {
    if token.is_empty() {
        anyhow::bail!("token is empty");
    }
    let app = Router::new()
        .route("/rpc", get(rpc))
        .with_state(AppState { token });
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Clone)]
struct AppState {
    token: String,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

async fn rpc(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if !tokens_match(&query.token, &state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(session).into_response()
}

async fn session(mut socket: WebSocket) {
    while let Some(message) = socket.recv().await {
        let Ok(message) = message else {
            break;
        };
        let Message::Text(text) = message else {
            continue;
        };
        let reply = {
            let mut handler = RefreshOnly;
            mailune_rpc::dispatch(&mut handler, &text)
        };
        if socket.send(Message::text(reply)).await.is_err() {
            break;
        }
    }
}

struct RefreshOnly;

impl Handler for RefreshOnly {
    fn handle(&mut self, method: Method) -> Result<Vec<Event>, mailune_rpc::Error> {
        match method {
            Method::Refresh => Ok(vec![Event::Notice {
                message: "refreshed".into(),
            }]),
            _ => Err(mailune_rpc::Error::Refused),
        }
    }
}

/// Compare without stopping at the first differing byte, and without logging either side.
fn tokens_match(provided: &str, expected: &str) -> bool {
    let provided = provided.as_bytes();
    let expected = expected.as_bytes();
    let mut mismatch = provided.len() ^ expected.len();
    let width = provided.len().max(expected.len());
    for index in 0..width {
        let left = provided.get(index).copied().unwrap_or(0);
        let right = expected.get(index).copied().unwrap_or(0);
        mismatch |= usize::from(left ^ right);
    }
    mismatch == 0
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    use super::serve;

    #[tokio::test]
    async fn refresh_answers_when_the_token_matches() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let token = "fixture-token";
        tokio::spawn(async move {
            serve(token.to_string(), listener).await.unwrap();
        });

        let rejected = connect_async(format!("ws://{addr}/rpc?token=nope")).await;
        assert!(rejected.is_err());

        let (mut socket, _) = connect_async(format!("ws://{addr}/rpc?token={token}"))
            .await
            .unwrap();
        socket
            .send(Message::text(
                r#"{"jsonrpc":"2.0","method":"refresh","id":1}"#,
            ))
            .await
            .unwrap();
        let reply = socket.next().await.unwrap().unwrap();
        let Message::Text(body) = reply else {
            panic!("expected text");
        };
        assert!(body.contains("refreshed"));
        assert!(!body.contains(token));
    }
}
