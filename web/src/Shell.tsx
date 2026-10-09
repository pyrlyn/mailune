import { useState } from "react";
import type { Address, ThreadRow } from "./contract.gen";
import { InboxIcon } from "./icons";
import { t } from "./i18n";
import "./shell.css";

/** Mailboxes in first-seen order, so the inbox the core lists first stays first. */
function mailboxesOf(threads: ThreadRow[]): string[] {
  return [...new Set(threads.map((thread) => thread.mailbox))];
}

function mailboxLabel(mailbox: string): string {
  return mailbox === "inbox" ? t("app.inbox") : mailbox;
}

function sender(address: Address): string {
  return address.name ?? address.email;
}

/** Mailboxes, the conversation list, and the open conversation. */
export function Shell({ threads }: { threads: ThreadRow[] }) {
  const mailboxes = mailboxesOf(threads);
  const [mailbox, setMailbox] = useState(mailboxes[0] ?? "inbox");
  const [openId, setOpenId] = useState<string | null>(null);
  const listed = threads.filter((thread) => thread.mailbox === mailbox);
  const open = threads.find((thread) => thread.id === openId) ?? null;

  return (
    <div className="shell">
      <nav className="pane" aria-label={t("app.mailboxes")}>
        <h1 className="brand">
          <InboxIcon />
          {t("app.mail")}
        </h1>
        <ul>
          {mailboxes.map((id) => (
            <li key={id}>
              <button
                type="button"
                aria-current={id === mailbox ? "page" : undefined}
                onClick={() => setMailbox(id)}
              >
                {mailboxLabel(id)}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <section className="pane" aria-label={t("app.conversations")}>
        <ul>
          {listed.map((thread) => (
            <li key={thread.id}>
              <button
                type="button"
                className="row"
                data-unread={thread.unread || undefined}
                aria-current={thread.id === openId ? "true" : undefined}
                onClick={() => setOpenId(thread.id)}
              >
                <span className="row-from">{sender(thread.from)}</span>
                <span className="row-stamp">{thread.stamp}</span>
                <span className="row-subject">{thread.subject}</span>
                <span className="row-snippet">{thread.snippet}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>
      <main className="pane" aria-label={t("app.message")}>
        {open ? (
          <article>
            <h2>{open.subject}</h2>
            <p className="meta">
              {sender(open.from)} · {open.from.email}
            </p>
            <p>{open.snippet}</p>
          </article>
        ) : (
          <div className="empty">
            <h2>{t("app.no_conversation_open")}</h2>
            <p>{t("app.pick_a_message_from")}</p>
          </div>
        )}
      </main>
    </div>
  );
}
