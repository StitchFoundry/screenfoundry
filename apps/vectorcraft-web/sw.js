// Service worker for the web app (#61). GitHub Pages sends `Cache-Control: max-age=600` for everything,
// so without this the browser re-checks the 80 MB .wasm every ten minutes. The files below carry a
// content hash in their names, so a cached copy is never stale: cache-first, and a new build simply
// has new names. Everything else (index.html, this file) goes to the network, so a deploy shows up
// on the next load.

const CACHE = "screenfoundry-hashed-v1";
// vectorcraft-web-<hash>.js, vectorcraft-web-<hash>_bg.wasm, snippets/<name>-<hash>/*.js, fonts/<sha16>/*
const HASHED = /\/(vectorcraft-web-[0-9a-f]+(_bg)?\.(js|wasm)|snippets\/[^/]+-[0-9a-f]+\/[^/]+\.js|fonts\/[0-9a-f]{16}\/[^/]+)$/;
const WASM = /\/vectorcraft-web-[0-9a-f]+_bg\.wasm$/;

self.addEventListener("install", () => self.skipWaiting());

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      for (const name of await caches.keys()) if (name !== CACHE && name.startsWith("screenfoundry-")) await caches.delete(name);
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin || !HASHED.test(url.pathname)) return;
  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      const hit = await cache.match(url.href);
      if (hit) return hit;
      const response = await fetch(request);
      if (response.ok && response.status === 200) {
        // Keep one app version: drop older wasm/js files when a new one arrives.
        const stored = cache.put(url.href, response.clone()).then(async () => {
          if (!WASM.test(url.pathname)) return;
          for (const old of await cache.keys()) {
            if (WASM.test(new URL(old.url).pathname) && old.url !== url.href) await cache.delete(old);
          }
        });
        event.waitUntil(stored.catch(() => {}));
      }
      return response;
    })(),
  );
});
