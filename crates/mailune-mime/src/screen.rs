//! Decide whether a sender is new.
//!
//! Comparison is the address only, case-insensitive. A display name is not
//! part of the key. An address with no earlier inbound message is screened
//! so the reader can treat it as untrusted until it shows up again.

/// Whether this address has written in before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SenderScreen {
    /// The address was on an earlier inbound message.
    Known,
    /// No prior inbound, so the reader should screen it.
    FirstTime,
}

/// `prior_inbound` is every address already seen on mail that arrived.
pub fn screen_sender(address: &str, prior_inbound: &[&str]) -> SenderScreen {
    let key = normalize(address);
    if key.is_empty() {
        return SenderScreen::FirstTime;
    }
    if prior_inbound.iter().any(|prior| normalize(prior) == key) {
        SenderScreen::Known
    } else {
        SenderScreen::FirstTime
    }
}

fn normalize(address: &str) -> String {
    let trimmed = address.trim();
    let inner = trimmed
        .rsplit_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map(|(email, _)| email.trim())
        .filter(|email| !email.is_empty())
        .unwrap_or(trimmed);
    inner.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_address_is_screened_and_a_repeat_is_not() {
        let prior = ["Ana@Example.com", "Bo <bo@example.com>"];
        assert_eq!(
            screen_sender("ana@example.com", &prior),
            SenderScreen::Known
        );
        assert_eq!(
            screen_sender("Bo <bo@example.com>", &prior),
            SenderScreen::Known
        );
        assert_eq!(
            screen_sender("Ana <ana@example.com>", &prior),
            SenderScreen::Known
        );
        assert_eq!(
            screen_sender("new@example.com", &prior),
            SenderScreen::FirstTime
        );
        assert_eq!(
            screen_sender("new@example.com", &[]),
            SenderScreen::FirstTime
        );
    }
}
