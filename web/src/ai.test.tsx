import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { emptyDraft } from "./Composer";
import { ShellView } from "./Shell";
import { assistResult, threads } from "./fixture";

describe("fixture AI", () => {
  it("shows a summary, reply chips, and an assist result", () => {
    const html = renderToStaticMarkup(
      <ShellView
        threads={threads}
        selectedId="thursday"
        onSelect={() => {}}
        draft={emptyDraft}
        onDraft={() => {}}
        confirmed={false}
        onConfirmed={() => {}}
      />,
    );
    expect(html).toContain("Grace wants to meet Thursday.");
    expect(html).toContain("Afternoon");
    expect(html).toContain(assistResult);
    expect(html).not.toContain("fetch(");
  });
});
