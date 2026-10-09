//! Contract crate: types that cross a crate boundary, and the traits that
//! are the only way out to the network, the disk, and the host.
//!
//! Domain code and adapters depend on this shape and not on each other.
//! Nothing here opens a socket, a file, or a process.

mod address;
mod envelope;
mod event;
mod flags;
mod host;
mod http;
mod ids;
mod mailbox;
mod submission;

pub use address::Address;
pub use envelope::{Envelope, TransportSecurity};
pub use event::{Category, Event, ThreadRow};
pub use flags::Flags;
pub use host::{
    AuthRequest, AuthResult, AuthSession, BackgroundScheduler, BackgroundWork, Clock, ConnId, Fs,
    ModelCapability, ModelPrompt, Net, NetworkPath, NetworkState, Notification, Notifier,
    PlatformModel, Secret, SecretId, SecretKind, SecretStore,
};
pub use http::{Http, HttpRequest, HttpResponse, Method};
pub use ids::{AccountId, MailboxId, MessageId, ThreadId};
pub use mailbox::MailboxRole;
pub use submission::Submission;

/// Failure returned by the contract crate.
///
/// Host traits return this. Callers already depend on it, so a new host
/// failure does not grow a second error enum. [`Error::Host`]'s message is
/// safe to log: it must not carry a token, password, database key, or
/// authorization code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// [`Fs::read`] was asked for a path the host does not have.
    #[error("not found: {path}")]
    NotFound {
        /// Path that was missing. Not a secret.
        path: String,
    },
    /// The host could not finish a call.
    ///
    /// `message` names the failure and nothing secret. Construct it with
    /// [`Error::host`].
    #[error("{operation}: {message}")]
    Host {
        /// Static label of the call, such as `net.connect`.
        operation: &'static str,
        /// What went wrong, with no secret material.
        message: String,
    },
}

impl Error {
    /// A host failure whose text is safe to log.
    ///
    /// `message` must not contain a token, password, database key, or
    /// authorization code.
    pub fn host(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Host {
            operation,
            message: message.into(),
        }
    }
}

/// Confirms the crate links. The types themselves do not fail.
pub fn ready() -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;

    use super::{Envelope, Event, Submission};

    #[test]
    fn ready_succeeds() {
        assert!(super::ready().is_ok());
    }

    #[test]
    fn contract_json_schema_matches_the_snapshot() {
        let schemas = serde_json::json!({
            "Envelope": schema_for!(Envelope),
            "Submission": schema_for!(Submission),
            "Event": schema_for!(Event),
        });
        insta::assert_json_snapshot!(schemas);
    }
}
