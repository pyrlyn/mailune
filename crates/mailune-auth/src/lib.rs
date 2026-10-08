//! Account discovery and PKCE from values the caller already has.
//!
//! Nothing here opens a socket, a browser, or a token store. DNS and HTTP stay
//! with the caller; SRV and MX results are values they pass in.

mod autoconfig;
mod lookup;
mod pkce;

pub use autoconfig::{
    Autoconfig, MxRecord, ServerEndpoint, ServerProtocol, SocketSecurity, SrvRecord, SrvService,
    endpoint_from_mx, endpoint_from_srv, parse_mozilla_xml, parse_well_known_json,
};
pub use lookup::{ConfigSource, discover};
pub use pkce::{Pkce, s256_challenge};

/// Failure returned by discovery.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The XML or JSON document, or a caller-supplied record, is not usable.
    #[error("autoconfig document is not valid: {0}")]
    Autoconfig(String),
    /// The PKCE verifier is the wrong length or uses a disallowed character.
    ///
    /// The message does not include the verifier.
    #[error("PKCE verifier is not valid: {0}")]
    Pkce(String),
    /// ISPDB, SRV, and MX produced no server.
    ///
    /// The text names the domain or the reason. It is not a password.
    #[error("autoconfig lookup failed: {0}")]
    Lookup(String),
}
