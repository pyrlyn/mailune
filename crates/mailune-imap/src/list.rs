//! LIST rows to [`MailboxRole`](mailune_protocol::MailboxRole).
//!
//! SPECIAL-USE (`\Sent`, `\Junk`, …) is the server's own label. A common
//! name is only a fallback, so a folder called "Archive" that the server
//! marked `\Trash` stays trash.

use imap_codec::ResponseCodec;
use imap_codec::decode::Decoder;
use imap_codec::imap_types::mailbox::Mailbox;
use imap_codec::imap_types::response::{Data, Response};
use mailune_protocol::MailboxRole;

use crate::Error;

/// One LIST row and the role it maps to, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedMailbox {
    /// Mailbox name. `INBOX` is the codec's inbox variant, not a raw atom.
    pub name: String,
    /// SPECIAL-USE or a common name. `None` for an ordinary folder.
    pub role: Option<MailboxRole>,
}

/// Map a LIST name and its attributes onto a mailbox role.
///
/// `attributes` are flag names as LIST prints them (`\Sent`). SPECIAL-USE
/// is checked before the name.
pub fn mailbox_role(name: &str, attributes: &[&str]) -> Option<MailboxRole> {
    for attribute in attributes {
        if let Some(role) = role_from_attribute(attribute) {
            return Some(role);
        }
    }
    role_from_name(name)
}

/// Decode LIST responses in `bytes` into mailbox rows.
///
/// Tagged status lines are skipped. A truncated response is an error.
pub fn parse_list(bytes: &[u8]) -> Result<Vec<ListedMailbox>, Error> {
    let codec = ResponseCodec::new();
    let mut rest = bytes;
    let mut listed = Vec::new();
    while !rest.is_empty() {
        let (next, response) = codec.decode(rest).map_err(|_| Error::Response)?;
        if let Response::Data(Data::List { items, mailbox, .. }) = response {
            let name = mailbox_name(&mailbox);
            let attributes: Vec<String> = items.iter().map(ToString::to_string).collect();
            let refs: Vec<&str> = attributes.iter().map(String::as_str).collect();
            listed.push(ListedMailbox {
                role: mailbox_role(&name, &refs),
                name,
            });
        }
        if next.len() == rest.len() {
            return Err(Error::Response);
        }
        rest = next;
    }
    Ok(listed)
}

fn mailbox_name(mailbox: &Mailbox<'_>) -> String {
    match mailbox {
        Mailbox::Inbox => "INBOX".to_string(),
        Mailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).into_owned(),
    }
}

fn role_from_attribute(attribute: &str) -> Option<MailboxRole> {
    match attribute
        .trim()
        .trim_start_matches('\\')
        .to_ascii_lowercase()
        .as_str()
    {
        "sent" => Some(MailboxRole::Sent),
        "drafts" => Some(MailboxRole::Drafts),
        "junk" => Some(MailboxRole::Spam),
        "trash" => Some(MailboxRole::Trash),
        "archive" => Some(MailboxRole::Archive),
        "all" => Some(MailboxRole::All),
        "flagged" => Some(MailboxRole::Starred),
        "important" => Some(MailboxRole::Important),
        _ => None,
    }
}

fn role_from_name(name: &str) -> Option<MailboxRole> {
    let leaf = name.rsplit(['/', '.']).next().unwrap_or(name);
    role_from_leaf(name).or_else(|| role_from_leaf(leaf))
}

fn role_from_leaf(name: &str) -> Option<MailboxRole> {
    match name.trim().to_ascii_lowercase().as_str() {
        "inbox" => Some(MailboxRole::Inbox),
        "sent" | "sent mail" | "sent items" | "sent messages" => Some(MailboxRole::Sent),
        "draft" | "drafts" => Some(MailboxRole::Drafts),
        "junk" | "junk mail" | "junk email" | "spam" => Some(MailboxRole::Spam),
        "trash" | "bin" | "deleted" | "deleted items" | "deleted messages" => {
            Some(MailboxRole::Trash)
        }
        "archive" | "archives" => Some(MailboxRole::Archive),
        "all" | "all mail" => Some(MailboxRole::All),
        "starred" | "flagged" => Some(MailboxRole::Starred),
        "important" => Some(MailboxRole::Important),
        "snoozed" => Some(MailboxRole::Snoozed),
        "scheduled" => Some(MailboxRole::Scheduled),
        "outbox" => Some(MailboxRole::Outbox),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mailune_protocol::MailboxRole;

    use super::{mailbox_role, parse_list};

    #[test]
    fn list_maps_special_use_and_common_names() {
        let bytes = concat!(
            r#"* LIST (\HasNoChildren) "/" INBOX"#,
            "\r\n",
            r#"* LIST (\HasNoChildren \Sent) "/" "[Gmail]/Sent Mail""#,
            "\r\n",
            r#"* LIST (\Junk) "/" Notes"#,
            "\r\n",
            r#"* LIST (\HasNoChildren) "/" "Projects""#,
            "\r\n",
            "A OK LIST completed\r\n",
        );
        let listed = parse_list(bytes.as_bytes()).unwrap();
        assert_eq!(listed.len(), 4);
        assert_eq!(listed[0].name, "INBOX");
        assert_eq!(listed[0].role, Some(MailboxRole::Inbox));
        assert_eq!(listed[1].name, "[Gmail]/Sent Mail");
        assert_eq!(listed[1].role, Some(MailboxRole::Sent));
        assert_eq!(listed[2].role, Some(MailboxRole::Spam));
        assert_eq!(listed[3].role, None);
    }

    #[test]
    fn a_common_name_is_used_when_the_server_sends_no_special_use() {
        assert_eq!(
            mailbox_role("INBOX.Sent Items", &[]),
            Some(MailboxRole::Sent)
        );
        assert_eq!(mailbox_role("Drafts", &[]), Some(MailboxRole::Drafts));
        assert_eq!(
            mailbox_role("Archive", &[r"\Trash"]),
            Some(MailboxRole::Trash)
        );
    }
}
