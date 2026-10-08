import { describe, expect, it } from "vitest";
import { translate } from "./i18n";

describe("catalogs", () => {
  it("loads English and German by language code", () => {
    expect(translate("en", "inbox")).toBe("Inbox");
    expect(translate("de", "inbox")).toBe("Posteingang");
  });

  it("falls back to English when the key is missing", () => {
    expect(translate("de", "title")).toBe("Mailune");
    expect(translate("fr", "inbox")).toBe("Inbox");
  });
});
