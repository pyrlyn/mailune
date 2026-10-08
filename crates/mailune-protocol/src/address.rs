//! A mailbox address. The UI mock also carries initials and a tint; those
//! are presentation and stay out of the contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Someone a message is from, to or copied to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Address {
    /// Display name when the header had one.
    pub name: Option<String>,
    /// Addr-spec (`ana@acme.io`).
    pub email: String,
}
