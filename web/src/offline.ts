// Network-first for same-origin GETs. Every good answer replaces the cached
// copy, so the cache holds the latest thread list and app shell, and serves
// them when the network fails. There is no push server: the cache refreshes
// whenever the app asks while online.

/** Bumped when the cached shape changes, so the old copy is dropped on activate. */
export const CACHE_NAME = "mailune-offline-v1";

/** The part of `Cache` this strategy uses, so a test can pass a map. */
export interface CacheLike {
  match(request: Request): Promise<Response | undefined>;
  put(request: Request, response: Response): Promise<void>;
}

/** Same-origin GETs only: another origin's answer is not ours to keep. */
export function shouldHandle(request: Request, origin: string): boolean {
  if (request.method !== "GET") {
    return false;
  }
  const url = new URL(request.url);
  return url.origin === origin && (url.protocol === "https:" || url.protocol === "http:");
}

function cacheable(response: Response): boolean {
  // `no-store` is the server saying this answer must not be kept on disk.
  return response.ok && !/\bno-store\b/i.test(response.headers.get("cache-control") ?? "");
}

/**
 * The network's answer, cached when it may be. When the network fails the
 * cached copy is served; with nothing cached the failure stands, so the
 * page sees an error rather than an empty list it would mistake for one.
 */
export async function networkFirst(
  request: Request,
  fetcher: (request: Request) => Promise<Response>,
  cache: CacheLike,
): Promise<Response> {
  try {
    const response = await fetcher(request);
    if (cacheable(response)) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await cache.match(request);
    if (cached) {
      return cached;
    }
    throw error;
  }
}
