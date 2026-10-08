//! Gmail API adapter: labels, threads, `history.list` diffs, and batch
//! fetches, sent through the injected [`mailune_protocol::Http`] transport.
//!
//! Gmail labels are mailboxes here: a message's label ids are the mailboxes
//! it is in, except `UNREAD` and `STARRED`, which become flags.

mod batch;
mod client;
mod model;
mod mutate;

pub use client::{GmailClient, SyncBatch};
pub use model::{GmailLabel, GmailMessage, HistoryDiff};
pub use mutate::{Draft, Sent};

/// Failure returned by the Gmail adapter. No variant carries the token.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The transport did not return a response.
    #[error(transparent)]
    Transport(#[from] mailune_protocol::Error),
    /// The API answered with a non-success status.
    #[error("Gmail API answered {status}")]
    Status {
        /// HTTP status code.
        status: u16,
    },
    /// A body is not the JSON or multipart shape this adapter expects.
    #[error("Gmail response is not valid: {0}")]
    Format(String),
}
