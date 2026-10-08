import { describe, expect, it } from "vitest";
import { threads } from "./fixture";
import { loadThreadList, memoryThreadCache } from "./offline";

describe("offline thread list", () => {
  it("serves the cached list when the network is offline", async () => {
    const cache = memoryThreadCache();
    const latest = JSON.stringify(threads);
    const cached = await loadThreadList(true, async () => latest, cache);
    expect(cached).toBe(latest);

    let networkCalls = 0;
    const offline = await loadThreadList(
      false,
      async () => {
        networkCalls += 1;
        throw new Error("offline");
      },
      cache,
    );
    expect(offline).toBe(latest);
    expect(networkCalls).toBe(0);
    expect(JSON.parse(offline)[1].subject).toBe("Thursday");
  });
});
