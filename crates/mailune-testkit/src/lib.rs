//! In-memory stand-ins for the nine host traits and the HTTP transport,
//! plus a submission script.
//!
//! The contract's own fake stays inside its tests, so another crate cannot
//! link it. Nothing here opens a socket, a file, or a keychain.

mod host;
mod http;
mod scenario;
mod synth;

pub use host::{FakeHost, poll_now};
pub use http::ScriptedHttp;
pub use scenario::Scenario;
pub use synth::{SyntheticMessage, SyntheticThread, ThreadKind, mailbox};
