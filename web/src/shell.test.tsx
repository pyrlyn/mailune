// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { Shell } from "./Shell";
import { fixtureThreads } from "./fixture";
import { t } from "./i18n";
import { click, mount, pane } from "./render.test-utils";

let cleanup: (() => Promise<void>) | undefined;
afterEach(async () => {
  await cleanup?.();
  cleanup = undefined;
});

async function shell() {
  const mounted = await mount(<Shell threads={fixtureThreads} />);
  cleanup = mounted.unmount;
  const { container } = mounted;
  return {
    mailboxes: pane(container, t("app.mailboxes")),
    list: pane(container, t("app.conversations")),
    reading: pane(container, t("app.message")),
  };
}

function rowNamed(list: HTMLElement, subject: string) {
  return [...list.querySelectorAll("button")].find((button) => button.textContent?.includes(subject));
}

describe("shell", () => {
  it("renders three panes over the fixture with nothing open", async () => {
    const { mailboxes, list, reading } = await shell();
    expect(mailboxes.textContent).toContain(t("app.inbox"));
    expect(list.querySelectorAll("button.row")).toHaveLength(2);
    expect(reading.querySelector("h2")?.textContent).toBe(t("app.no_conversation_open"));
  });

  it("shows the chosen row in the reading pane", async () => {
    const { list, reading } = await shell();
    await click(rowNamed(list, "Thursday"));
    expect(reading.querySelector("h2")?.textContent).toBe("Thursday");
    expect(reading.textContent).toContain("grace@example.com");
    expect(rowNamed(list, "Thursday")?.getAttribute("aria-current")).toBe("true");
    expect(rowNamed(list, "Build notes")?.hasAttribute("aria-current")).toBe(false);

    await click(rowNamed(list, "Build notes"));
    expect(reading.querySelector("h2")?.textContent).toBe("Build notes");
    expect(rowNamed(list, "Thursday")?.hasAttribute("aria-current")).toBe(false);
  });

  it("lists only the chosen mailbox and keeps the open thread", async () => {
    const { mailboxes, list, reading } = await shell();
    await click(rowNamed(list, "Thursday"));
    await click(rowNamed(mailboxes, "archive"));
    expect(list.querySelectorAll("button.row")).toHaveLength(1);
    expect(rowNamed(list, "Invoice 2026-10")).toBeDefined();
    expect(reading.querySelector("h2")?.textContent).toBe("Thursday");
  });
});
