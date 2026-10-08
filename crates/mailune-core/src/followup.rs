//! Follow-up detection.
//!
//! A message the person sent is awaiting a reply when nothing later has come
//! back from that address. The check only looks at the list it is given.

use mailune_protocol::MessageId;

/// One message in a correspondence. `at` is an order the caller already knows
/// (a timestamp or a sequence). This module does not read a clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    /// Message id.
    pub id: MessageId,
    /// Position in time. A larger value is later.
    pub at: u64,
    /// The person sent this message.
    pub sent: bool,
    /// The other party. Compared case-insensitively, surrounding spaces ignored.
    pub correspondent: String,
}

/// Sent messages that have no later inbound from the same correspondent.
///
/// An inbound message with the same `at` does not count: equal stamps are not
/// an order, so only a strictly later inbound clears the follow-up.
pub fn awaiting_reply(messages: &[Exchange]) -> Vec<MessageId> {
    messages
        .iter()
        .filter(|message| message.sent && !answered(messages, message))
        .map(|message| message.id.clone())
        .collect()
}

fn answered(messages: &[Exchange], sent: &Exchange) -> bool {
    messages.iter().any(|candidate| {
        !candidate.sent
            && candidate.at > sent.at
            && same_address(&candidate.correspondent, &sent.correspondent)
    })
}

fn same_address(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

#[cfg(test)]
mod tests {
    use mailune_protocol::MessageId;

    use super::{Exchange, awaiting_reply};

    fn message(id: &str, at: u64, sent: bool, correspondent: &str) -> Exchange {
        Exchange {
            id: MessageId::new(id),
            at,
            sent,
            correspondent: correspondent.into(),
        }
    }

    #[test]
    fn a_sent_message_with_no_later_inbound_is_awaiting() {
        let messages = vec![
            message("out", 1, true, "Ada@Acme.io"),
            message("other", 2, false, "bea@acme.io"),
            message("earlier", 0, false, "ada@acme.io"),
        ];
        let waiting = awaiting_reply(&messages);
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].as_str(), "out");
    }

    #[test]
    fn a_later_inbound_from_that_address_clears_only_the_earlier_send() {
        let messages = vec![
            message("first", 1, true, "ada@acme.io"),
            message("reply", 2, false, " ada@acme.io "),
            message("second", 3, true, "ada@acme.io"),
        ];
        let waiting = awaiting_reply(&messages);
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].as_str(), "second");
    }
}
