import { describe, expect, it } from "vitest";
import { decodePayload, encodePayload, type SaveDraftPayload } from "./payload";

describe("save_draft payload", () => {
  it("round-trips", () => {
    const payload: SaveDraftPayload = {
      to: [{ name: "Ada", email: "ada@example.com" }],
      subject: "Hello",
      body: "See you Thursday",
    };
    expect(decodePayload(encodePayload(payload))).toEqual(payload);
  });
});
