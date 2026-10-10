// A local mailbox for the shell until mailune-server (E1) answers over the
// socket. Rows use the generated contract type, so a field the core renames
// breaks the build here too.

import type { ThreadRow } from "./contract.gen";
import { fixtureBody, type SanitizedHtml } from "./message";

type RowFields = Pick<ThreadRow, "id" | "from" | "subject" | "snippet" | "stamp" | "mailbox">;

function row(fields: RowFields & Partial<ThreadRow>): ThreadRow {
  return {
    account: "local",
    message_count: 1,
    unread: false,
    flagged: false,
    important: false,
    pinned: false,
    snoozed: false,
    draft: false,
    has_attachment: false,
    category: "primary",
    labels: [],
    ...fields,
  };
}

export const fixtureThreads: ThreadRow[] = [
  row({
    id: "build",
    from: { name: "Ada Lovelace", email: "ada@example.com" },
    subject: "Build notes",
    snippet: "The nightly build is ready for review.",
    stamp: "09:30",
    mailbox: "inbox",
    unread: true,
    message_count: 3,
  }),
  row({
    id: "thursday",
    from: { name: "Grace Hopper", email: "grace@example.com" },
    subject: "Thursday",
    snippet: "Can we move the review to Thursday afternoon?",
    stamp: "Yesterday",
    mailbox: "inbox",
    flagged: true,
  }),
  row({
    id: "invoice",
    from: { name: null, email: "billing@example.com" },
    subject: "Invoice 2026-10",
    snippet: "Your invoice for October is attached.",
    stamp: "Oct 2",
    mailbox: "archive",
    has_attachment: true,
  }),
];

export const fixtureBodies: Readonly<Record<string, SanitizedHtml>> = { build: fixtureBody };
