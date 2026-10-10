// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { Composer, draftFrom, draftSubmission, type Draft } from "./Composer";
import { Shell } from "./Shell";
import type { Submission } from "./contract.gen";
import { fixtureThreads } from "./fixture";
import { t } from "./i18n";
import { click, mount, pane, type } from "./render.test-utils";
import { validator } from "./schema.test-utils";

let cleanup: (() => Promise<void>) | undefined;
afterEach(async () => {
  await cleanup?.();
  cleanup = undefined;
});

const saved = {
  to: [
    { name: "Ada Lovelace", email: "ada@example.com" },
    { name: null, email: "grace@example.com" },
  ],
  subject: "Build notes",
  body: "The build is ready.\nSee you Thursday.",
};

async function composer(initial?: Draft) {
  const onDraft = vi.fn<(submission: Submission) => void>();
  const onSend = vi.fn<(submission: Submission) => void>();
  const mounted = await mount(<Composer initial={initial} onDraft={onDraft} onSend={onSend} />);
  cleanup = mounted.unmount;
  const form = mounted.container.querySelector("form");
  const field = (name: string) => form?.querySelector<HTMLInputElement>(`[name="${name}"]`);
  return {
    onDraft,
    onSend,
    field,
    send: () => form?.querySelector("button[type=submit]") as HTMLButtonElement,
    form,
  };
}

describe("composer", () => {
  it("round-trips recipients, subject and body through save_draft", async () => {
    expect(validator("Submission")({ save_draft: saved })).toBe(true);
    const draft = draftFrom(saved);
    expect(draft.to).toBe("Ada Lovelace <ada@example.com>, grace@example.com");
    expect(draftSubmission(draft)).toEqual({ save_draft: saved });

    const { field, onDraft } = await composer(draft);
    expect(field("to")?.value).toBe(draft.to);
    expect(field("subject")?.value).toBe("Build notes");
    expect(field("body")?.value).toBe(saved.body);

    await type(field("subject"), "Build notes v2");
    const last = onDraft.mock.lastCall?.[0];
    expect(last).toEqual({ save_draft: { ...saved, subject: "Build notes v2" } });
    expect(validator("Submission")(last)).toBe(true);
  });

  it("keeps send disabled until the confirm box is on", async () => {
    const { field, send, onSend } = await composer(draftFrom(saved));
    expect(send().disabled).toBe(true);
    await click(send());
    expect(onSend).not.toHaveBeenCalled();

    await click(field("confirm"));
    expect(field("confirm")?.checked).toBe(true);
    expect(send().disabled).toBe(false);
    await click(send());
    expect(onSend).toHaveBeenCalledTimes(1);
    const sent = onSend.mock.lastCall?.[0];
    expect(sent).toEqual({ send: saved });
    expect(validator("Submission")(sent)).toBe(true);
    expect(send().disabled).toBe(true);
  });

  it("turns the confirmation off when the message changes", async () => {
    const { field, send, onSend, form } = await composer(draftFrom(saved));
    await click(field("confirm"));
    await type(field("to"), "mallory@example.com");
    expect(field("confirm")?.checked).toBe(false);
    expect(send().disabled).toBe(true);
    form?.requestSubmit();
    expect(onSend).not.toHaveBeenCalled();
  });

  it("cannot send to nobody even when confirmed", async () => {
    const { field, send } = await composer();
    await type(field("subject"), "Hi");
    await click(field("confirm"));
    expect(send().disabled).toBe(true);
  });

  it("opens from the shell", async () => {
    const mounted = await mount(<Shell threads={fixtureThreads} />);
    cleanup = mounted.unmount;
    const mailboxes = pane(mounted.container, t("app.mailboxes"));
    await click([...mailboxes.querySelectorAll("button")].find((b) => b.textContent === t("app.compose")));
    const reading = pane(mounted.container, t("app.message"));
    expect(reading.querySelector("form.composer")).not.toBeNull();
    expect(reading.textContent).toContain(t("compose.confirm_recipients"));
  });
});
