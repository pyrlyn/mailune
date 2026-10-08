//! Opaque ids. Providers disagree on the spelling (IMAP UID, JMAP id, Graph
//! id), so the contract stores the string and does not pick an algorithm.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a provider id. The contents are not inspected.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrows the provider id.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id! {
    /// One mail account.
    AccountId
}

string_id! {
    /// One mailbox, special-use or user-created.
    MailboxId
}

string_id! {
    /// One conversation, as the provider or the threading engine names it.
    ThreadId
}

string_id! {
    /// One message.
    MessageId
}
