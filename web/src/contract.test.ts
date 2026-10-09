import { readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";
import { contractSchemas, renderContract } from "../scripts/contract.ts";
import type { Event, Submission } from "./contract.gen";
import { t } from "./i18n";

function validator(root: "Event" | "Submission") {
  return new Ajv2020({ strict: false }).compile(contractSchemas()[root]);
}

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
});

describe("UI text", () => {
  it("comes from the gettext catalog by msgctxt key", () => {
    expect(t("compose.send")).toBe("Send");
    expect(t("no.such.key")).toBe("no.such.key");
  });
});
