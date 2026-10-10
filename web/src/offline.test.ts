import { describe, expect, it } from "vitest";
import type { Event, ThreadRow } from "./contract.gen";
import { fixtureThreads } from "./fixture";
import { type CacheLike, networkFirst, shouldHandle } from "./offline";

const ORIGIN = "http://mailune.test";
const THREADS = `${ORIGIN}/threads`;

/** A Cache keyed by URL, enough for the strategy. */
function memoryCache(): CacheLike & { size: () => number } {
  const entries = new Map<string, Response>();
  return {
    size: () => entries.size,
    match: async (request) => entries.get(request.url)?.clone(),
    put: async (request, response) => {
      entries.set(request.url, response);
    },
  };
}

/** A network the test can take offline, as a browser does: fetch rejects. */
function network() {
  let online = true;
  let answer: Response = new Response("not set", { status: 500 });
  return {
    offline: () => {
      online = false;
    },
    answer: (response: Response) => {
      answer = response;
    },
    fetch: async (_request: Request) => {
      if (!online) {
        throw new TypeError("Failed to fetch");
      }
      return answer.clone();
    },
  };
}

function snapshot(threads: ThreadRow[], headers?: HeadersInit): Response {
  const event: Event = { snapshot: { threads } };
  return new Response(JSON.stringify(event), { headers: { "content-type": "application/json", ...headers } });
}

describe("offline thread list", () => {
  it("serves the latest list the network gave once it goes offline", async () => {
    const net = network();
    const cache = memoryCache();
    net.answer(snapshot(fixtureThreads.slice(0, 1)));
    await networkFirst(new Request(THREADS), net.fetch, cache);
    net.answer(snapshot(fixtureThreads));
    const online = (await (await networkFirst(new Request(THREADS), net.fetch, cache)).json()) as Event;
    net.offline();
    const offline = (await (await networkFirst(new Request(THREADS), net.fetch, cache)).json()) as Event;
    expect(offline).toEqual(online);
    expect(offline).toEqual({ snapshot: { threads: fixtureThreads } });
  });

  it("fails offline when nothing was cached, instead of showing an empty list", async () => {
    const net = network();
    net.offline();
    await expect(networkFirst(new Request(THREADS), net.fetch, memoryCache())).rejects.toThrow("Failed to fetch");
  });

  it("keeps the last good list when the server errors or forbids storing", async () => {
    const net = network();
    const cache = memoryCache();
    net.answer(snapshot(fixtureThreads));
    await networkFirst(new Request(THREADS), net.fetch, cache);
    net.answer(new Response("down", { status: 503 }));
    expect((await networkFirst(new Request(THREADS), net.fetch, cache)).status).toBe(503);
    net.answer(snapshot([], { "cache-control": "private, no-store" }));
    await networkFirst(new Request(THREADS), net.fetch, cache);
    net.offline();
    const served = (await (await networkFirst(new Request(THREADS), net.fetch, cache)).json()) as Event;
    expect(served).toEqual({ snapshot: { threads: fixtureThreads } });
  });

  it("handles only same-origin GETs", () => {
    expect(shouldHandle(new Request(THREADS), ORIGIN)).toBe(true);
    expect(shouldHandle(new Request(THREADS, { method: "POST", body: "{}" }), ORIGIN)).toBe(false);
    expect(shouldHandle(new Request("http://elsewhere.test/threads"), ORIGIN)).toBe(false);
  });
});
