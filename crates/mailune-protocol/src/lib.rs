//! Contract crate: types that cross a crate boundary.
//!
//! Domain code and adapters depend on this shape and not on each other.
//! Nothing here opens a socket, a file or a process. Host traits land later.

mod address;
mod envelope;
mod event;
mod flags;
mod ids;
mod mailbox;
mod submission;

pub use address::Address;
pub use envelope::{Envelope, TransportSecurity};
pub use event::{Category, Event, ThreadRow};
pub use flags::Flags;
pub use ids::{AccountId, MailboxId, MessageId, ThreadId};
pub use mailbox::MailboxRole;
pub use submission::Submission;

/// Failure returned by the contract crate.
///
/// Variants arrive with fallible constructors. The enum exists so callers
/// already depend on this crate's error.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

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
