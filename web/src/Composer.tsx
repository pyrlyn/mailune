import { useState, type ChangeEvent, type FormEvent } from "react";
import type { Address, Submission } from "./contract.gen";
import { t } from "./i18n";

/** What the person typed. `to` is the recipient field as text. */
export interface Draft {
  to: string;
  subject: string;
  body: string;
}

export const emptyDraft: Draft = { to: "", subject: "", body: "" };

type SaveDraft = Extract<Submission, { save_draft: unknown }>["save_draft"];
type Send = Extract<Submission, { send: unknown }>["send"];

const NAMED = /^(.*?)\s*<([^<>]+)>$/;

/** `Ada <ada@example.com>, bob@example.com` as addresses; empty entries are dropped. */
export function parseRecipients(text: string): Address[] {
  return text
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry !== "")
    .map((entry) => {
      const named = NAMED.exec(entry);
      const name = named?.[1]?.trim();
      return named?.[2]
        ? { name: name ? name : null, email: named[2].trim() }
        : { name: null, email: entry };
    });
}

export function formatRecipients(to: Address[]): string {
  return to.map((address) => (address.name ? `${address.name} <${address.email}>` : address.email)).join(", ");
}

export function draftSubmission(draft: Draft): Submission {
  const save_draft: SaveDraft = { to: parseRecipients(draft.to), subject: draft.subject, body: draft.body };
  return { save_draft };
}

export function draftFrom(saved: SaveDraft): Draft {
  return { to: formatRecipients(saved.to), subject: saved.subject, body: saved.body };
}

function sendSubmission(draft: Draft): Submission {
  const send: Send = { to: parseRecipients(draft.to), subject: draft.subject, body: draft.body };
  return { send };
}

/**
 * A new message. Every change is offered as a `save_draft` submission. Send
 * stays disabled until the confirm box is on, and any edit turns it off, so
 * what leaves is what the person confirmed.
 */
export function Composer({
  initial = emptyDraft,
  onDraft,
  onSend,
}: {
  initial?: Draft;
  onDraft?: (submission: Submission) => void;
  onSend: (submission: Submission) => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [confirmed, setConfirmed] = useState(false);
  const canSend = confirmed && parseRecipients(draft.to).length > 0;

  const edit =
    (field: keyof Draft) => (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const next = { ...draft, [field]: event.target.value };
      setDraft(next);
      setConfirmed(false);
      onDraft?.(draftSubmission(next));
    };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    // Enter in a field submits the form even while the button is disabled.
    if (canSend) {
      onSend(sendSubmission(draft));
      setConfirmed(false);
    }
  };

  return (
    <form className="composer" onSubmit={submit}>
      <label>
        {t("compose.to")}
        <input name="to" type="text" autoComplete="off" value={draft.to} onChange={edit("to")} />
      </label>
      <label>
        {t("compose.subject")}
        <input name="subject" type="text" value={draft.subject} onChange={edit("subject")} />
      </label>
      <label>
        {t("compose.write_your_message")}
        <textarea name="body" rows={10} value={draft.body} onChange={edit("body")} />
      </label>
      <label className="confirm">
        <input
          name="confirm"
          type="checkbox"
          checked={confirmed}
          onChange={(event) => setConfirmed(event.target.checked)}
        />
        {t("compose.confirm_recipients")}
      </label>
      <button type="submit" disabled={!canSend}>
        {t("compose.send")}
      </button>
    </form>
  );
}
