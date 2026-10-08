// Fixture JMAP `Mailbox/query`. The shell shows these ids; the WASM client
// is a later task.

export interface MailboxQueryBody {
  accountId: string;
  ids: string[];
}

export interface JmapResponse {
  methodResponses: ["Mailbox/query", MailboxQueryBody, string][];
}

export const mailboxQuery: JmapResponse = {
  methodResponses: [
    ["Mailbox/query", { accountId: "ada", ids: ["mb-inbox", "mb-sent"] }, "0"],
  ],
};

export function mailboxIds(response: JmapResponse): string[] {
  const ids: string[] = [];
  for (const [name, body] of response.methodResponses) {
    if (name === "Mailbox/query") {
      ids.push(...body.ids);
    }
  }
  return ids;
}
