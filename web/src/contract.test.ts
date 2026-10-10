import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { renderContract } from "../scripts/contract.ts";
import type { Event, Submission } from "./contract.gen";
import { t } from "./i18n";
import { validator } from "./schema.test-utils";

describe("generated contract types", () => {
  it("match the mailune-protocol schema snapshot", async () => {
    const committed = readFileSync(new URL("./contract.gen.ts", import.meta.url), "utf8");
    expect(committed).toBe(await renderContract());
  });

  it("round-trip an event the core would send", () => {
    const event: Event = {
      snapshot: {
        threads: [
          {
            id: "t1",
            account: "local",
            from: { name: "Ada", email: "ada@example.com" },
            subject: "Build notes",
            snippet: "The build is ready.",
            stamp: "09:30",
            message_count: 2,
            unread: true,
            flagged: false,
            important: false,
            pinned: false,
            snoozed: false,
            draft: false,
            has_attachment: false,
            category: "primary",
            mailbox: "inbox",
            labels: ["work"],
          },
        ],
      },
    };
    const decoded: unknown = JSON.parse(JSON.stringify(event));
    const validate = validator("Event");
    expect(validate(decoded), JSON.stringify(validate.errors)).toBe(true);
    expect(decoded).toEqual(event);
  });

  it("reject what the contract does not define", () => {
    const validate = validator("Submission");
    const draft: Submission = { save_draft: { to: [], subject: "Hi", body: "" } };
    expect(validate(draft)).toBe(true);
    expect(validate({ send_now: {} })).toBe(false);
    expect(validate({ save_draft: { to: "ada@example.com", subject: "Hi", body: "" } })).toBe(false);
  });

  it("reject a count Rust cannot hold", () => {
    const validate = validator("Event");
    const row = (message_count: number) => ({
      snapshot: {
        threads: [
          {
            id: "t1",
            account: "local",
            from: { name: null, email: "ada@example.com" },
            subject: "",
            snippet: "",
            stamp: "",
            message_count,
            unread: false,
            flagged: false,
            important: false,
            pinned: false,
            snoozed: false,
            draft: false,
            has_attachment: false,
            category: "primary",
            mailbox: "inbox",
            labels: [],
          },
        ],
      },
    });
    expect(validate(row(1))).toBe(true);
    expect(validate(row(-1))).toBe(false);
    expect(validate(row(1.5))).toBe(false);
  });
});

describe("UI text", () => {
  it("comes from the gettext catalog by msgctxt key", () => {
    expect(t("compose.send")).toBe("Send");
    expect(t("no.such.key")).toBe("no.such.key");
  });
});
