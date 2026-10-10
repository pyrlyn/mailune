//! `mailune-server`: the native core behind JSON-RPC over WebSocket, for the
//! web client of a self-hosted install.
//!
//! The address comes from `MAILUNE_SERVER_ADDR` (default loopback only).
//! The bearer token is random per start and printed once to stderr: it is
//! never written to disk or the environment, because secrets live only in
//! the OS keychain and a headless host may not have one.

mod server;

use anyhow::Context;
use mailune_protocol::Event;
use mailune_rpc::{Error, Handler, Method};

/// Environment variable that overrides the listen address.
const ADDR_VAR: &str = "MAILUNE_SERVER_ADDR";
/// Loopback by default, so a first start is not reachable from the network.
const DEFAULT_ADDR: &str = "127.0.0.1:8484";

/// Stands in until the app runtime exposes an rpc handler: every method is
/// refused, so a client sees a JSON-RPC error rather than made-up data.
struct Unwired;

impl Handler for Unwired {
    fn handle(&mut self, _method: Method) -> Result<Vec<Event>, Error> {
        Err(Error::Refused)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var(ADDR_VAR).unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("cannot listen on {addr}"))?;
    let token = server::Token::generate();
    eprintln!(
        // Plain ws is the loopback default; a non-loopback install needs TLS
        // terminated in front of it, which is outside this binary.
        "mailune-server: ws://{}{}\nmailune-server: token {}", // nosemgrep
        listener.local_addr()?,
        server::RPC_PATH,
        token.reveal()
    );
    server::serve(listener, token, Unwired)
        .await
        .context("server stopped")
}
