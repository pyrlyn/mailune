//! Exchange Web Services adapter for on-premises Exchange Server.
//!
//! Operations and their SOAP form come from Thunderbird's `ews` crate; this
//! crate sends them through the injected [`mailune_protocol::Http`]
//! transport and maps the answers onto protocol types. Exchange Online is
//! refused: it is reached through Microsoft Graph (`mailune-graph`).

mod client;

pub use client::{EwsClient, EwsMessage, SyncPage};

/// Failure returned by the EWS adapter. No variant carries the token.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The transport did not return a response.
    #[error(transparent)]
    Transport(#[from] mailune_protocol::Error),
    /// The server answered with a non-success status.
    #[error("EWS server answered {status}")]
    Status {
        /// HTTP status code.
        status: u16,
    },
    /// The SOAP document could not be built or read.
    #[error("EWS document is not valid: {0}")]
    Soap(String),
    /// The server answered the operation with an error response code.
    #[error("EWS operation failed: {code}")]
    Response {
        /// The EWS response code, such as `ErrorInvalidSyncStateData`.
        code: String,
    },
    /// The endpoint is not an https URL, or is Exchange Online.
    #[error("EWS endpoint refused: {0}")]
    Endpoint(&'static str),
}
