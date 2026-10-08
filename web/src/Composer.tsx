export interface Draft {
  to: string;
  subject: string;
  body: string;
}

export const emptyDraft: Draft = { to: "", subject: "", body: "" };

export function updateDraft(draft: Draft, field: keyof Draft, value: string): Draft {
  return { ...draft, [field]: value };
}

export function Composer({
  draft,
  confirmed,
  onDraft,
  onConfirmed,
}: {
  draft: Draft;
  confirmed: boolean;
  onDraft: (draft: Draft) => void;
  onConfirmed: (confirmed: boolean) => void;
}) {
  return (
    <form
      className="composer"
      onSubmit={(event) => {
        event.preventDefault();
      }}
    >
      <label>
        To
        <input
          name="to"
          value={draft.to}
          onChange={(event) => onDraft(updateDraft(draft, "to", event.target.value))}
        />
      </label>
      <label>
        Subject
        <input
          name="subject"
          value={draft.subject}
          onChange={(event) => onDraft(updateDraft(draft, "subject", event.target.value))}
        />
      </label>
      <label>
        Body
        <textarea
          name="body"
          value={draft.body}
          onChange={(event) => onDraft(updateDraft(draft, "body", event.target.value))}
        />
      </label>
      <label>
        <input
          type="checkbox"
          checked={confirmed}
          onChange={(event) => onConfirmed(event.target.checked)}
        />
        Confirm send
      </label>
      <button type="submit" disabled={!confirmed}>
        Send
      </button>
    </form>
  );
}
