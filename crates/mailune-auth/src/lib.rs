//! Account discovery from documents the caller already has.
//!
//! Nothing here opens a socket. DNS and HTTP stay with the caller; SRV and MX
//! results are values they pass in.

mod autoconfig;

pub use autoconfig::{
    Autoconfig, MxRecord, ServerEndpoint, ServerProtocol, SocketSecurity, SrvRecord, SrvService,
    endpoint_from_mx, endpoint_from_srv, parse_mozilla_xml, parse_well_known_json,
};

/// Failure returned by discovery.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The XML or JSON document, or a caller-supplied record, is not usable.
    #[error("autoconfig document is not valid: {0}")]
    Autoconfig(String),
}
