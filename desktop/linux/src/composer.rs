//! Composer description.
//!
//! Recipient, subject, and body live in `share/composer.desc`. Send stays
//! off until confirm is required. This file only reads that description.

use crate::Error;

/// A draft the composer can show. Send is closed until the person confirms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composer {
    /// To address.
    pub recipient: String,
    /// Subject line.
    pub subject: String,
    /// Body text.
    pub body: String,
    /// Send is armed only when this is true.
    pub send: bool,
    /// Confirm must be present before send can turn on.
    pub confirm: bool,
}

/// Reads the draft fields and the send gate from `text`.
///
/// # Errors
///
/// [`Error::Description`] when a field is missing, or send is on without confirm.
pub fn load_composer(text: &str) -> Result<Composer, Error> {
    let mut recipient = None;
    let mut subject = None;
    let mut body = None;
    let mut send = None;
    let mut confirm = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(' ') else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        match key {
            "recipient" => recipient = Some(value.to_string()),
            "subject" => subject = Some(value.to_string()),
            "body" => body = Some(value.to_string()),
            "send" => send = Some(value == "on"),
            "confirm" => confirm = Some(value == "required"),
            _ => {}
        }
    }
    match (recipient, subject, body, send, confirm) {
        (Some(recipient), Some(subject), Some(body), Some(send), Some(confirm))
            if !recipient.is_empty()
                && !subject.is_empty()
                && !body.is_empty()
                && !(send && !confirm) =>
        {
            Ok(Composer {
                recipient,
                subject,
                body,
                send,
                confirm,
            })
        }
        _ => Err(Error::Description),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::load_composer;

    #[test]
    fn send_stays_off_until_confirm() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/composer.desc");
        let text = std::fs::read_to_string(path).unwrap();
        let composer = load_composer(&text).unwrap();
        assert_eq!(composer.recipient, "ada@example.com");
        assert_eq!(composer.subject, "Hello");
        assert_eq!(composer.body, "See you at the dock");
        assert!(!composer.send);
        assert!(composer.confirm);
    }
}
