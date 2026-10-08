import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { emptyDraft } from "./Composer";
import { ShellView } from "./Shell";
import { threads } from "./fixture";
import { mailboxIds, mailboxQuery } from "./jmap";

describe("JMAP mailbox query", () => {
  it("shows the fixture mailbox ids", () => {
    const ids = mailboxIds(mailboxQuery);
    const html = renderToStaticMarkup(
      <ShellView
        threads={threads}
        selectedId={threads[0].id}
        onSelect={() => {}}
        draft={emptyDraft}
        onDraft={() => {}}
        confirmed={false}
        onConfirmed={() => {}}
      />,
    );
    expect(ids).toEqual(["mb-inbox", "mb-sent"]);
    expect(html).toContain("mb-inbox");
    expect(html).toContain("mb-sent");
  });
});
