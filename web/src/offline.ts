// The latest thread list, stored by the service worker. Tests use the same
// read path with the network marked offline. There is no push server.

export const threadListPath = "/threads.json";

export interface ThreadListCache {
  read(): Promise<string | undefined>;
  write(body: string): Promise<void>;
}

export function memoryThreadCache(): ThreadListCache {
  let body: string | undefined;
  return {
    async read() {
      return body;
    },
    async write(next) {
      body = next;
    },
  };
}

export async function loadThreadList(
  online: boolean,
  network: () => Promise<string>,
  cache: ThreadListCache,
): Promise<string> {
  if (online) {
    const body = await network();
    await cache.write(body);
    return body;
  }
  const cached = await cache.read();
  if (cached === undefined) {
    throw new Error("thread list is not cached");
  }
  return cached;
}
