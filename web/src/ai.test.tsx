// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { Composer, draftFrom } from "./Composer";
import { Shell } from "./Shell";
import { fixtureAi, fixtureAssist, type ThreadAi } from "./ai";
import { fixtureThreads } from "./fixture";
import { t } from "./i18n";
import { click, mount, pane } from "./render.test-utils";

let cleanup: (() => Promise<void>) | undefined;
afterEach(async () => {
  await cleanup?.();
  cleanup = undefined;
});

async function shell(ai: Readonly<Record<string, ThreadAi>> = fixtureAi) {
  const mounted = await mount(<Shell threads={fixtureThreads} ai={ai} assist={fixtureAssist} />);
  cleanup = mounted.unmount;
  const list = pane(mounted.container, t("app.conversations"));
  const reading = pane(mounted.container, t("app.message"));
  const open = (subject: string) =>
    click([...list.querySelectorAll("button")].find((button) => button.textContent?.includes(subject)));
  return { reading, open };
}

describe("AI surfaces", () => {
  it("show the summary and three reply chips for the open thread", async () => {
    const { reading, open } = await shell();
    await open("Thursday");
    const summary = pane(reading, t("thread.summary"));
    expect(summary.querySelector("p")?.textContent).toBe(fixtureAi.thursday?.summary.text);
    const chips = [...pane(reading, t("thread.suggested_replies")).querySelectorAll("button")];
    expect(chips.map((chip) => chip.textContent)).toEqual([...(fixtureAi.thursday?.replies ?? [])]);
  });

  it("start a reply draft from a chip without sending it", async () => {
    const { reading, open } = await shell();
    await open("Thursday");
    await click(pane(reading, t("thread.suggested_replies")).querySelector("button"));
    const form = reading.querySelector("form.composer");
    const field = (name: string) => form?.querySelector<HTMLInputElement>(`[name="${name}"]`);
    expect(field("to")?.value).toBe("grace@example.com");
    expect(field("subject")?.value).toBe("Re: Thursday");
    expect(field("body")?.value).toBe("Thursday works.");
    expect(field("confirm")?.checked).toBe(false);
    expect(form?.querySelector<HTMLButtonElement>("button[type=submit]")?.disabled).toBe(true);
  });

  it("render model text as text, not markup", async () => {
    const hostile = "<img src=x onerror=alert(1)>Read me";
    const { reading, open } = await shell({
      thursday: { summary: { kind: "short", text: hostile }, replies: [hostile, "b", "c"] },
    });
    await open("Thursday");
    expect(reading.querySelector("img")).toBeNull();
    expect(pane(reading, t("thread.summary")).querySelector("p")?.textContent).toBe(hostile);
  });

  it("show an assist result in the composer and apply it only on request", async () => {
    const draft = draftFrom({ to: [{ name: null, email: "ada@example.com" }], subject: "Build", body: "long text" });
    const mounted = await mount(<Composer initial={draft} assist={fixtureAssist} onSend={() => {}} />);
    cleanup = mounted.unmount;
    const assist = pane(mounted.container, t("compose.suggested"));
    expect(assist.querySelector("p")?.textContent).toBe(fixtureAssist.text);
    const body = mounted.container.querySelector<HTMLTextAreaElement>('[name="body"]');
    expect(body?.value).toBe("long text");

    const confirm = mounted.container.querySelector<HTMLInputElement>('[name="confirm"]');
    await click(confirm);
    await click(assist.querySelector("button"));
    expect(body?.value).toBe(fixtureAssist.text);
    expect(confirm?.checked).toBe(false);
  });
});
