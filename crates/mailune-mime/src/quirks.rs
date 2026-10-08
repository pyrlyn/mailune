//! Provider facts a later IMAP client can query. This table does not connect.
//!
//! Opened 2026-10-08:
//! - Gmail extensions: <https://developers.google.com/workspace/gmail/imap/imap-extensions>
//! - Gmail servers: <https://developers.google.com/workspace/gmail/imap/imap-smtp>
//! - Gmail in another client: <https://support.google.com/mail/answer/7126229?hl=en>
//! - iCloud: <https://support.apple.com/en-us/102525>
//! - Yahoo: <https://help.yahoo.com/kb/SLN4075.html>
//! - Outlook.com: <https://support.microsoft.com/en-us/office/pop-imap-and-smtp-settings-for-outlook-com-d088b986-291d-42b8-9564-9c414e2aa040>

/// A provider this table knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Gmail.
    Gmail,
    /// iCloud Mail.
    Icloud,
    /// Yahoo Mail.
    Yahoo,
    /// Outlook.com / Hotmail.
    OutlookCom,
}

/// How the TCP connection is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Implicit TLS on connect (the vendor's "SSL").
    Tls,
    /// Plain greeting, then STARTTLS.
    StartTls,
}

/// How the account signs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    /// OAuth 2 / XOAUTH2 / Modern Auth.
    Oauth2,
    /// An app-specific password, not the account password.
    AppPassword,
}

/// What the IMAP `LOGIN` username is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImapUsername {
    /// The full address.
    FullAddress,
    /// Usually the local part. The full address is the fallback Apple names.
    LocalPart,
}

/// One host the table records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    /// DNS name.
    pub host: &'static str,
    /// TCP port.
    pub port: u16,
    /// TLS style for this port.
    pub transport: Transport,
}

/// A mailbox the server marks with a special-use attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialMailbox {
    /// Mailbox name as `LIST` returns it.
    pub name: &'static str,
    /// Special-use attribute, including the leading backslash (`\All`).
    pub attribute: &'static str,
}

/// One row of the provider table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quirks {
    /// Which provider.
    pub provider: Provider,
    /// IMAP endpoint.
    pub imap: Endpoint,
    /// SMTP endpoints, in the order the vendor lists them.
    pub smtp: &'static [Endpoint],
    /// Sign-in the vendor documents for a third-party client.
    pub auth: Auth,
    /// IMAP username shape.
    pub imap_username: ImapUsername,
    /// Gmail exposes labels as IMAP folders and as `X-GM-LABELS`.
    pub labels_as_folders: bool,
    /// Extra IMAP attributes and capabilities named by the vendor.
    pub imap_attributes: &'static [&'static str],
    /// Special-use mailboxes named by the vendor. Empty when the page does not list them.
    pub special_mailboxes: &'static [SpecialMailbox],
    /// `Some(false)` when the vendor says POP is unsupported. `None` when the page did not say.
    pub pop: Option<bool>,
    /// Facts that do not fit a field, still from the pages above.
    pub notes: &'static [&'static str],
}

const GMAIL_SMTP: &[Endpoint] = &[
    Endpoint {
        host: "smtp.gmail.com",
        port: 465,
        transport: Transport::Tls,
    },
    Endpoint {
        host: "smtp.gmail.com",
        port: 587,
        transport: Transport::StartTls,
    },
];

const GMAIL_ATTRS: &[&str] = &[
    "X-GM-EXT-1",
    "X-GM-LABELS",
    "X-GM-MSGID",
    "X-GM-THRID",
    "X-GM-RAW",
];

const GMAIL_MAILBOXES: &[SpecialMailbox] = &[
    SpecialMailbox {
        name: "[Gmail]/All Mail",
        attribute: "\\All",
    },
    SpecialMailbox {
        name: "[Gmail]/Drafts",
        attribute: "\\Drafts",
    },
    SpecialMailbox {
        name: "[Gmail]/Important",
        attribute: "\\Important",
    },
    SpecialMailbox {
        name: "[Gmail]/Sent Mail",
        attribute: "\\Sent",
    },
    SpecialMailbox {
        name: "[Gmail]/Spam",
        attribute: "\\Junk",
    },
    SpecialMailbox {
        name: "[Gmail]/Starred",
        attribute: "\\Flagged",
    },
    SpecialMailbox {
        name: "[Gmail]/Trash",
        attribute: "\\Trash",
    },
];

const GMAIL_NOTES: &[&str] = &[
    "CAPABILITY includes X-GM-EXT-1. Labels are folders, and also X-GM-LABELS on FETCH, STORE, and SEARCH.",
    "XLIST was deprecated in 2013. \\Important is Gmail's extra special-use attribute.",
    "IMAP sessions last about 24 hours. An OAuth session lasts about the access token, usually 1 hour.",
    "Personal accounts keep IMAP on (the Enable/Disable toggle went away in January 2025). App passwords are not the recommended sign-in.",
    "smtp.gmail.com: 465 is SSL, 587 is TLS. IMAP and POP require SSL (imap.gmail.com:993, pop.gmail.com:995).",
];

const ICLOUD_SMTP: &[Endpoint] = &[Endpoint {
    host: "smtp.mail.me.com",
    port: 587,
    transport: Transport::StartTls,
}];

const ICLOUD_NOTES: &[&str] = &[
    "IMAP username is usually the local part (johnappleseed, not the full address). If that fails, try the full address.",
    "SMTP username is the full address. Both sides use an app-specific password.",
    "SSL is required. If SSL errors, try TLS. SMTP port is 587, and Apple also names STARTTLS.",
    "iCloud Mail does not support POP.",
];

const YAHOO_SMTP: &[Endpoint] = &[
    Endpoint {
        host: "smtp.mail.yahoo.com",
        port: 465,
        transport: Transport::Tls,
    },
    Endpoint {
        host: "smtp.mail.yahoo.com",
        port: 587,
        transport: Transport::StartTls,
    },
];

const YAHOO_NOTES: &[&str] = &[
    "IMAP is imap.mail.yahoo.com port 993 with SSL. SMTP is smtp.mail.yahoo.com port 465 or 587, SSL and authentication required.",
    "The login is the full address. The password is an app password.",
];

const OUTLOOK_SMTP: &[Endpoint] = &[Endpoint {
    host: "smtp-mail.outlook.com",
    port: 587,
    transport: Transport::StartTls,
}];

const OUTLOOK_NOTES: &[&str] = &[
    "Modern Auth / OAuth2 is required. The username is the email address.",
    "IMAP is outlook.office365.com port 993 with SSL/TLS. SMTP is smtp-mail.outlook.com port 587 with STARTTLS.",
    "The page also says incoming and outgoing servers are the same; the settings table names two hosts, and this row follows the table.",
    "POP and IMAP are off until the user enables them under Settings, Mail, Forwarding and IMAP.",
];

const GMAIL: Quirks = Quirks {
    provider: Provider::Gmail,
    imap: Endpoint {
        host: "imap.gmail.com",
        port: 993,
        transport: Transport::Tls,
    },
    smtp: GMAIL_SMTP,
    auth: Auth::Oauth2,
    imap_username: ImapUsername::FullAddress,
    labels_as_folders: true,
    imap_attributes: GMAIL_ATTRS,
    special_mailboxes: GMAIL_MAILBOXES,
    pop: Some(true),
    notes: GMAIL_NOTES,
};

const ICLOUD: Quirks = Quirks {
    provider: Provider::Icloud,
    imap: Endpoint {
        host: "imap.mail.me.com",
        port: 993,
        transport: Transport::Tls,
    },
    smtp: ICLOUD_SMTP,
    auth: Auth::AppPassword,
    imap_username: ImapUsername::LocalPart,
    labels_as_folders: false,
    imap_attributes: &[],
    special_mailboxes: &[],
    pop: Some(false),
    notes: ICLOUD_NOTES,
};

const YAHOO: Quirks = Quirks {
    provider: Provider::Yahoo,
    imap: Endpoint {
        host: "imap.mail.yahoo.com",
        port: 993,
        transport: Transport::Tls,
    },
    smtp: YAHOO_SMTP,
    auth: Auth::AppPassword,
    imap_username: ImapUsername::FullAddress,
    labels_as_folders: false,
    imap_attributes: &[],
    special_mailboxes: &[],
    pop: None,
    notes: YAHOO_NOTES,
};

const OUTLOOK: Quirks = Quirks {
    provider: Provider::OutlookCom,
    imap: Endpoint {
        host: "outlook.office365.com",
        port: 993,
        transport: Transport::Tls,
    },
    smtp: OUTLOOK_SMTP,
    auth: Auth::Oauth2,
    imap_username: ImapUsername::FullAddress,
    labels_as_folders: false,
    imap_attributes: &[],
    special_mailboxes: &[],
    pop: Some(true),
    notes: OUTLOOK_NOTES,
};

const ALL: &[&Quirks] = &[&GMAIL, &ICLOUD, &YAHOO, &OUTLOOK];

/// Every row, Gmail then iCloud then Yahoo then Outlook.com.
pub fn all() -> &'static [&'static Quirks] {
    ALL
}

/// The row for `provider`.
pub fn quirks(provider: Provider) -> &'static Quirks {
    match provider {
        Provider::Gmail => &GMAIL,
        Provider::Icloud => &ICLOUD,
        Provider::Yahoo => &YAHOO,
        Provider::OutlookCom => &OUTLOOK,
    }
}

/// The row whose IMAP host is `host`, ASCII case-insensitive.
pub fn by_imap_host(host: &str) -> Option<&'static Quirks> {
    let host = host.trim();
    ALL.iter()
        .copied()
        .find(|row| row.imap.host.eq_ignore_ascii_case(host))
}

impl Quirks {
    /// Whether the vendor names this IMAP attribute or capability.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.imap_attributes
            .iter()
            .any(|attr| attr.eq_ignore_ascii_case(name))
    }

    /// Mailbox name for a special-use attribute (`\All`), when the vendor listed one.
    pub fn special_mailbox(&self, attribute: &str) -> Option<&'static str> {
        self.special_mailboxes
            .iter()
            .find(|mailbox| mailbox.attribute.eq_ignore_ascii_case(attribute))
            .map(|mailbox| mailbox.name)
    }
}

#[cfg(test)]
mod tests {
    use super::{Auth, ImapUsername, Provider, Transport, by_imap_host, quirks};

    #[test]
    fn hosts_select_the_documented_row() {
        let icloud = by_imap_host("IMAP.MAIL.ME.COM").unwrap();
        assert_eq!(icloud.provider, Provider::Icloud);
        assert_eq!(icloud.imap.port, 993);
        assert_eq!(icloud.imap.transport, Transport::Tls);
        assert_eq!(icloud.auth, Auth::AppPassword);
        assert_eq!(icloud.imap_username, ImapUsername::LocalPart);
        assert_eq!(icloud.pop, Some(false));
        assert_eq!(icloud.smtp[0].host, "smtp.mail.me.com");
        assert_eq!(icloud.smtp[0].port, 587);

        let gmail = quirks(Provider::Gmail);
        assert!(gmail.labels_as_folders);
        assert!(gmail.has_attribute("X-GM-LABELS"));
        assert_eq!(gmail.special_mailbox("\\All"), Some("[Gmail]/All Mail"));
        assert_eq!(gmail.imap.host, "imap.gmail.com");
        assert_eq!(gmail.smtp.len(), 2);

        let outlook = quirks(Provider::OutlookCom);
        assert_eq!(outlook.auth, Auth::Oauth2);
        assert_eq!(outlook.imap.host, "outlook.office365.com");
        assert_eq!(outlook.smtp[0].host, "smtp-mail.outlook.com");
        assert_eq!(outlook.smtp[0].transport, Transport::StartTls);

        let yahoo = quirks(Provider::Yahoo);
        assert_eq!(yahoo.smtp.len(), 2);
        assert_eq!(yahoo.pop, None);

        assert!(by_imap_host("imap.example").is_none());
    }
}
