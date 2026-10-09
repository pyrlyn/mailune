import { useState } from "react";
import { Composer } from "./Composer";
import type { Address, Submission, ThreadRow } from "./contract.gen";
import { Reader } from "./Reader";
import { InboxIcon } from "./icons";
import { t } from "./i18n";
import type { SanitizedHtml } from "./message";
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
export function Shell({
  threads,
  bodies = {},
  onSubmit = () => {},
}: {
  threads: ThreadRow[];
  bodies?: Readonly<Record<string, SanitizedHtml>>;
  onSubmit?: (submission: Submission) => void;
}) {
  const mailboxes = mailboxesOf(threads);
  const [mailbox, setMailbox] = useState(mailboxes[0] ?? "inbox");
  const [openId, setOpenId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const listed = threads.filter((thread) => thread.mailbox === mailbox);
  const open = threads.find((thread) => thread.id === openId) ?? null;
  const body = open ? bodies[open.id] : undefined;

  return (
    <div className="shell">
      <nav className="pane" aria-label={t("app.mailboxes")}>
        <h1 className="brand">
          <InboxIcon />
          {t("app.mail")}
        </h1>
        <button type="button" className="compose" onClick={() => setComposing(true)}>
          {t("app.compose")}
        </button>
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
                onClick={() => {
                  setOpenId(thread.id);
                  setComposing(false);
                }}
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
        {composing ? (
          <Composer
            onDraft={onSubmit}
            onSend={(submission) => {
              onSubmit(submission);
              setComposing(false);
            }}
          />
        ) : open ? (
          <article>
            <h2>{open.subject}</h2>
            <p className="meta">
              {sender(open.from)} · {open.from.email}
            </p>
            {body ? <Reader body={body} /> : <p>{open.snippet}</p>}
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
