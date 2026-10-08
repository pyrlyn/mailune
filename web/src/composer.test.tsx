import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Composer, updateDraft, type Draft } from "./Composer";

const draft: Draft = {
  to: "ada@example.com",
  subject: "Hello",
  body: "See you Thursday",
};

describe("composer", () => {
  it("round-trips recipient, subject, and body", () => {
    const next = updateDraft(
      updateDraft(updateDraft(draft, "to", "grace@example.com"), "subject", "Next"),
      "body",
      "Afternoon",
    );
    expect(next).toEqual({
      to: "grace@example.com",
      subject: "Next",
      body: "Afternoon",
    });
    const html = renderToStaticMarkup(
      <Composer draft={draft} confirmed={false} assist="" onDraft={() => {}} onConfirmed={() => {}} />,
    );
    expect(html).toContain('value="ada@example.com"');
    expect(html).toContain('value="Hello"');
    expect(html).toContain("See you Thursday");
  });

  it("keeps send disabled until confirm is on", () => {
    const off = renderToStaticMarkup(
      <Composer draft={draft} confirmed={false} assist="" onDraft={() => {}} onConfirmed={() => {}} />,
    );
    expect(off).toContain('disabled=""');
    const on = renderToStaticMarkup(
      <Composer draft={draft} confirmed={true} assist="" onDraft={() => {}} onConfirmed={() => {}} />,
    );
    expect(on).not.toContain('disabled=""');
    expect(on).toContain("Send");
  });
});
