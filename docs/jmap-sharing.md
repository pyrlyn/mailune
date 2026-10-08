# Shared inboxes and comments

Checked 2026-10-08. This is a design note. It does not add a server, a protocol client, or a comment store.

## Sources

- RFC 9670, JMAP Sharing, N. Jenkins (ed.), November 2024, updates RFC 8620. https://www.rfc-editor.org/rfc/rfc9670 and https://www.rfc-editor.org/info/rfc9670 — text fetched 2026-10-08.
- RFC 8621, JMAP Mail, checked the same day at https://www.rfc-editor.org/rfc/rfc8621.txt for whether `Mailbox` already has `shareWith`. It does not. The text does contain `isSubscribed`, `myRights`, and `mayReadItems`.

There is no Mailune server on this path. A client that implements the design below talks only to the person's mail provider, over the provider's JMAP session.

## What RFC 9670 actually defines

Section 1 says the specification is a framework for sharing and that it does not define what may be shared or how fine the permissions are. The types it does define are `Principal` (Section 2) and `ShareNotification` (Section 3).

Section 1.4 separates permission from subscription. The worked case is a company that grants access to many mailboxes while each person subscribes to a few. For servers that implement this RFC, the Session `accounts` object includes an account only when the user owns it or is subscribed to at least one record in it. `Principal/query` still lists principals the user may access, including accounts they have not subscribed to. The principal object carries an account id for those.

Section 1.5 adds `urn:ietf:params:jmap:principals` and `urn:ietf:params:jmap:principals:owner`. The owner capability points at the account that holds the `Principal` and at that principal's id.

A `ShareNotification` records a rights change: who changed it, `objectType`, `objectAccountId`, `objectId`, `oldRights`, `newRights`, and a display `name`. Section 3 gives `"Mailbox"` as an example of `objectType`, next to `"Calendar"`. The name is so a person whose access was removed can still see what they lost. `ShareNotification/set` allows only destroy; create and update are forbidden.

Section 4 says a shareable data type must define three properties:

- `isSubscribed`
- `myRights`, a map of permission name to boolean, with the names defined by that data type
- `shareWith`, null when the object is not shared, otherwise a map from `Principal` id to a rights object in the same shape as `myRights`

The owner of the account must not appear in `shareWith`. Section 4.1 shows the shape on a fictional `TodoList`, not on a mailbox. The client finds principals through `Principal/get` on the account named by `urn:ietf:params:jmap:principals:owner`, then updates `shareWith` with a `/set` on the shareable type.

## Shared inboxes

A shared inbox fits this framework only as a mailbox type that references Section 4 and adds `shareWith`. RFC 9670 does not add that property itself. It only:

- uses mailboxes as the subscription example in Section 1.4
- allows a `ShareNotification` to name `objectType` `"Mailbox"`

RFC 8621 already has `Mailbox.isSubscribed` and `Mailbox.myRights` (including `mayReadItems`) and has no `shareWith`. So the provider's mailboxes are not shareable under RFC 9670 until a binding defines `Mailbox.shareWith` and the right names.

When that binding exists, the client design is:

1. Read the session. Show an account in the sidebar when the user owns it or is subscribed to at least one record (Section 1.4). Do not invent a second store for those mailboxes.
2. Treat a mailbox with a non-null `shareWith` as shared. The keys are principal ids. The values are the rights that binding defines. The owner's principal is absent.
3. To change sharing, `Principal/get` on the account from `urn:ietf:params:jmap:principals:owner`, then `Mailbox/set` updating `shareWith`. Offer a principal only when that principal's capability says the user may share with them (the `mayShareWith` flag in the Section 4.1 example).
4. Show `ShareNotification` objects with `objectType` `"Mailbox"` as "your access changed", using `name`, `oldRights`, and `newRights`. Destroying one only clears the notice.
5. Reading and sending stay on the provider's `Email` and `EmailSubmission` methods in the account that holds the mailbox. Nothing is copied to a Mailune host.

Until a binding publishes `Mailbox.shareWith`, Mailune does not send that property and does not pretend an inbox is shared.

## Comments

RFC 9670 cannot carry comments. It defines no comment type and no comment method. The only time the word "comment" appears in the document is the heading "Request for Comments". `ShareNotification` is a rights-change record, and the client may not create one. A thread comment, an internal note, or an assignment has nowhere to go in this RFC.

Putting comments on a Mailune server would leave the provider path. This design does not do that. Comments wait for a data type that actually stores them.

## Decision

Shared inboxes are the Section 4 properties on `Mailbox`: subscribe through the session rules, share by `shareWith` keyed on `Principal` id, and learn about changes from `ShareNotification`. RFC 9670 describes that frame and does not itself add `shareWith` to mail, so the client waits for the mailbox binding. Comments are outside the RFC.
