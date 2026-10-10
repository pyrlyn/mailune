/// <reference lib="webworker" />
// The service worker: built to /sw.js and registered by main.tsx in
// production. The strategy lives in offline.ts so vitest can drive it.

import { CACHE_NAME, networkFirst, shouldHandle } from "./offline";

declare const self: ServiceWorkerGlobalScope;

self.addEventListener("install", () => {
  void self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      for (const name of await caches.keys()) {
        if (name !== CACHE_NAME) {
          await caches.delete(name);
        }
      }
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  if (!shouldHandle(event.request, self.location.origin)) {
    return;
  }
  event.respondWith(
    caches.open(CACHE_NAME).then((cache) => networkFirst(event.request, (request) => fetch(request), cache)),
  );
});
