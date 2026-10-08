//! In-memory ledger of model calls.
//!
//! A row names the feature, the provider, the bytes sent, the message ids,
//! and a retention class. The class is a label only: this module never opens
//! a store, so dropping the ledger drops the rows.

use mailune_protocol::MessageId;

use crate::Feature;

/// How long a caller may keep a row. This crate does not keep it past the
/// process, whichever class is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retention {
    /// Forgotten when the ledger is dropped.
    Ephemeral,
    /// The person may clear it later. Still not written here.
    UntilCleared,
}

/// One cloud or local call the router already decided to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowRecord {
    /// Feature that spent the bytes.
    pub feature: Feature,
    /// Provider name. Not a secret and not a key.
    pub provider: String,
    /// Bytes handed to that provider.
    pub bytes: u64,
    /// Messages those bytes came from.
    pub messages: Vec<MessageId>,
    /// How long a later store may keep this row.
    pub retention: Retention,
}

/// Append-only rows for this process.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    rows: Vec<FlowRecord>,
}

impl Ledger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Appends `row`. Order is the order of calls.
    pub fn record(&mut self, row: FlowRecord) {
        self.rows.push(row);
    }

    /// Rows recorded so far, oldest first.
    pub fn rows(&self) -> &[FlowRecord] {
        &self.rows
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::MessageId;

    use super::{FlowRecord, Ledger, Retention};
    use crate::Feature;

    #[test]
    fn a_row_keeps_feature_provider_bytes_messages_and_retention() {
        let mut ledger = Ledger::new();
        assert!(ledger.rows().is_empty());
        ledger.record(FlowRecord {
            feature: Feature::Summarize,
            provider: "platform".into(),
            bytes: 42,
            messages: vec![MessageId::new("m1"), MessageId::new("m2")],
            retention: Retention::Ephemeral,
        });
        ledger.record(FlowRecord {
            feature: Feature::DraftReply,
            provider: "local".into(),
            bytes: 7,
            messages: vec![MessageId::new("m3")],
            retention: Retention::UntilCleared,
        });
        let rows = ledger.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].feature, Feature::Summarize);
        assert_eq!(rows[0].provider, "platform");
        assert_eq!(rows[0].bytes, 42);
        assert_eq!(rows[0].messages[0].as_str(), "m1");
        assert_eq!(rows[0].messages[1].as_str(), "m2");
        assert_eq!(rows[0].retention, Retention::Ephemeral);
        assert_eq!(rows[1].feature, Feature::DraftReply);
        assert_eq!(rows[1].retention, Retention::UntilCleared);
    }
}
