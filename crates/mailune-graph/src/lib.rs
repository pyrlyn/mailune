//! Microsoft Graph mail adapter: folders, per-folder message delta queries
//! with `$select`, and mutations (PATCH, move, sendMail, `$batch`), sent
//! through the injected [`mailune_protocol::Http`] transport. Exchange
//! Online is reached this way; EWS is a separate path.
//!
//! Paging and delta links come from the server. They are followed only when
//! they point back at the Graph host, so the token never goes elsewhere.

mod client;
mod model;
mod mutate;

pub use client::{FolderDelta, GraphClient};
pub use model::{GraphFolder, GraphMessage};
pub use mutate::{BatchOutcome, MessagePatch};

/// Failure returned by the Graph adapter. No variant carries the token.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The transport did not return a response.
    #[error(transparent)]
    Transport(#[from] mailune_protocol::Error),
    /// Graph answered with a non-success status.
    #[error("Microsoft Graph answered {status}")]
    Status {
        /// HTTP status code.
        status: u16,
    },
    /// A body is not the JSON shape this adapter expects.
    #[error("Microsoft Graph response is not valid: {0}")]
    Format(String),
    /// A paging or delta link points away from the Graph host.
    #[error("Microsoft Graph link is not on the Graph host")]
    ForeignLink,
}
