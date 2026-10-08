import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { emptyDraft } from "./Composer";
import { ShellView } from "./Shell";
import { chosenThread, threads } from "./fixture";

describe("thread list", () => {
  it("shows the thread for the chosen row", () => {
    let selectedId = threads[0].id;
    const onSelect = (id: string) => {
      selectedId = id;
    };
    onSelect(chosenThread(threads, "thursday").id);
    const html = renderToStaticMarkup(
      <ShellView
        threads={threads}
        selectedId={selectedId}
        onSelect={onSelect}
        draft={emptyDraft}
        onDraft={() => {}}
        confirmed={false}
        onConfirmed={() => {}}
      />,
    );
    expect(html).toContain('data-pane="folders"');
    expect(html).toContain('data-pane="list"');
    expect(html).toContain('data-pane="reading"');
    expect(html).toContain("<h2>Thursday</h2>");
    expect(html).toContain('aria-pressed="true"');
    expect(html).not.toContain("<h2>Build notes</h2>");
  });
});
