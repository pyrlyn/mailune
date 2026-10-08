# Shared inboxes and comments on JMAP Sharing (P34)

Can Mailune offer shared inboxes and comments, as Spark and Missive do, with no Mailune server in the path? Short answer:

- **Shared inboxes: yes.** Reading works on RFC 8621 alone. RFC 9670 adds the people (Principals) and the notices (ShareNotifications). Changing who a mailbox is shared with needs the JMAP Mail Sharing draft, which a server announces with a capability.
- **Comments: RFC 9670 cannot carry them.** It is a framework for making data types shareable, and no JMAP data type for comments exists. Comments travel as ordinary Emails in a shared mailbox instead (see below).

Every fact below was checked on 2026-10-08 against the source linked next to it.

## Sources

| Fact | Source |
| --- | --- |
| RFC 9670 (November 2024, Standards Track, updates RFC 8620) defines the Principal and ShareNotification data types and a framework for shared data. It defines no mail-specific type. | https://www.rfc-editor.org/rfc/rfc9670.txt, sections 1.3, 2, 3, 4 |
| A shareable data type MUST define `isSubscribed`, `myRights` and `shareWith`. `shareWith` maps Principal ids to rights, and the owner MUST NOT be in it. | RFC 9670, section 4 |
| A Principal has `type` (individual, group, resource, location, other), `name`, `email`, `timeZone` and `accounts`. `Principal/query` lists the Principals the user can see. | RFC 9670, sections 2 and 2.4 |
| The Session MUST list only Accounts the user owns or where the user is subscribed to at least one record. Push StateChanges SHOULD cover only subscribed data. | RFC 9670, section 1.4 |
| A ShareNotification is created by the server when the user's rights change. It carries `changedBy`, `objectType`, `objectId`, `oldRights`, `newRights` and `name`. The client dismisses it with `/set` destroy. | RFC 9670, section 3 |
| Security: Principal names can be spoofed with look-alike names, sharing can turn a brief compromise into a lasting one, and the set of Principals must be tightly controlled. | RFC 9670, sections 6.1, 6.2, 6.4 |
| RFC 8621 Mailboxes already have `myRights` (MailboxRights, mapped from IMAP ACLs per RFC 4314) and `isSubscribed`, which SHOULD default to false in shared accounts. Permissions are per mailbox: "another user's inbox has been shared as read-only". | https://www.rfc-editor.org/rfc/rfc8621.txt, sections 1 and 2 |
| RFC 8621 has no `shareWith` on Mailbox. | RFC 8621, section 2 (property list) |
| `draft-ietf-jmap-mail-sharing-02` (29 July 2026, Stalwart Labs, expires 30 January 2027) adds `Mailbox.shareWith` and the `mayShare` right (IMAP ACL `a`), announced by the `urn:ietf:params:jmap:mail:share` capability. It is an Internet-Draft, so it is work in progress. | https://www.ietf.org/archive/id/draft-ietf-jmap-mail-sharing-02.txt, sections 1.2.1 and 2 |
| `maySetKeywords` covers every keyword except `$seen`. Keywords are shared with IMAP, and RFC 8621 warns that a shared keyword may disclose what a user did with a message. | RFC 8621, sections 2 and 4.1.1, and the keyword registrations in section 10.4 |
| No IETF document or draft defines a JMAP data type for comments or annotations on an Email. The datatracker lists JMAP drafts for sharing, mail sharing, calendars and email push, and none for comments. | https://datatracker.ietf.org/api/v1/doc/document/?name__contains=jmap&type=draft |

## Shared inboxes

What the client does, all against the user's own JMAP server:

1. **Find shared mailboxes.** Read the Session `accounts`. An account the user does not own appears once they are subscribed to something in it. To find everything the user *may* see, call `Principal/query`, then follow each Principal's `accounts`.
2. **Show who owns what.** For each account, the `urn:ietf:params:jmap:principals:owner` account capability gives the owning `principalId`. Show the Principal's `name` and `email`, never a name alone. RFC 9670 section 6.1 is about look-alike names, so the address is the identity.
3. **Subscribe.** Set `isSubscribed` on the mailbox. Mailune lists subscribed shared mailboxes in the sidebar and keeps a separate "browse shared" screen for the rest, as RFC 9670 section 1.4 suggests.
4. **Respect rights.** Grey out actions that `myRights` does not grant: archive and delete need `mayRemoveItems`, flags need `maySetKeywords`, and read state needs `maySetSeen` on *all* the mailboxes of the Email.
5. **Manage sharing.** Show the sharing editor only when the account has `urn:ietf:params:jmap:mail:share` and the mailbox grants `mayShare`. Otherwise sharing is set up in the server's own admin UI, and Mailune only reads it. A new share is confirmed in the app, like send and delete, because of RFC 9670 section 6.2.
6. **Notices.** `ShareNotification/query` feeds a "shared with you" banner. Dismissing it destroys the notification.

The local store already keys data by account (`docs/architecture.md`), so a shared account syncs like any other account. Nothing here needs a Mailune server, and the push relay never sees the content.

**Assignment and status** (Missive's "assigned to Ada", "done") fit in Email keywords, for example `$mailune-assigned-<principalId>` and `$mailune-done`. Two limits:

- Whether a non-system keyword set by one user is visible to another depends on the server. RFC 8621 does not require keywords to be shared per mailbox. **Unverified** for Stalwart and Dovecot. Check with the R8 compose servers before building on it.
- A keyword is visible to every user of the mailbox and to IMAP clients. That is the point here, but the UI says so.

## Comments

RFC 9670 cannot carry comments. It only defines how a data type becomes shareable, and no data type for comments on an Email exists in an RFC or a current draft. A Mailune-only JMAP extension would need a Mailune server, which is out of scope.

The path that works on any server that shares mailboxes, JMAP or IMAP:

- **A comment is an Email.** It goes into a `Comments` mailbox next to the shared inbox, shared with the same Principals. It has `In-Reply-To` and `References` pointing at the commented message's `Message-ID`, `From` set to the commenter, and a `$mailune-comment` keyword. It is created with `Email/set` or `Email/import`.
- **Mailune folds comments into the thread view** by `In-Reply-To`. They are never shown as mail and never sent over SMTP: they are written straight into the mailbox.
- **Edit and delete** are a new Email that replaces the old one, plus destroying the old one, if the user has `mayRemoveItems`. There is no edit history beyond what the server keeps.

Costs, recorded so the creator can decide:

- Other clients show comments as messages in a `Comments` folder. They degrade to readable mail, which is the honest fallback.
- Comments are as private as the mailbox. Anyone who can read the shared inbox can read its comments.
- Comments on an encrypted message are plain mail unless Mailune encrypts them to the same recipients. The rule that encrypted mail never goes to a cloud model applies to such a comment as well.

## Decision

- Build shared inboxes on RFC 8621 shared accounts and RFC 9670 Principals and ShareNotifications. Offer sharing management only behind `urn:ietf:params:jmap:mail:share`, and keep it a draft-dependent feature until the draft is an RFC.
- Build comments as Emails in a shared `Comments` mailbox, as above. Do not wait for a JMAP comments type, since none is being drafted.
- Before either is planned: check keyword sharing and the mail-sharing capability against the R8 Stalwart and Dovecot servers.
