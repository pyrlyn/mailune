import { loadThreadList, threadListPath, type ThreadListCache } from "./offline";

const cacheName = "mailune-threads";

const stored: ThreadListCache = {
  async read() {
    const match = await caches.match(threadListPath);
    if (!match) {
      return undefined;
    }
    return match.text();
  },
  async write(body) {
    const box = await caches.open(cacheName);
    await box.put(threadListPath, new Response(body));
  },
};

const scope = self as unknown as {
  addEventListener: (
    type: "fetch",
    handler: (event: {
      request: Request;
      respondWith: (response: Promise<Response>) => void;
    }) => void,
  ) => void;
};

scope.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.pathname !== threadListPath) {
    return;
  }
  event.respondWith(serve(event.request));
});

async function serve(request: Request): Promise<Response> {
  const body = await loadThreadList(
    navigator.onLine,
    async () => {
      const response = await fetch(request);
      if (!response.ok) {
        throw new Error("thread list request failed");
      }
      return response.text();
    },
    stored,
  );
  return new Response(body, { headers: { "content-type": "application/json" } });
}
