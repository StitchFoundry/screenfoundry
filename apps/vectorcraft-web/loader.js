// Trunk initializer (`data-initializer` in index.html): shows the download on the loading screen and
// registers the cache-first service worker (sw.js). Trunk streams the .wasm through this and hands
// the Response to WebAssembly.instantiateStreaming, so compiling overlaps the download (#61).
//
// This is plain JavaScript because it has to run before the wasm exists; keep it small.

const mb = (n) => (n / 1048576).toFixed(1);

// First visit: the page isn't controlled by the service worker yet (it is installed while the wasm
// downloads), so the worker can't cache that download. Keep the hashed .wasm in the same Cache
// Storage the worker reads, written from the page as it arrives. Later visits are served by the
// worker (sw.js), which also catches the browser's <link rel=preload> for the wasm.
const CACHE = "screenfoundry-hashed-v1";
const WASM = /\/vectorcraft-web-[0-9a-f]+_bg\.wasm$/;
if ("caches" in window) {
  const realFetch = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    let url;
    try {
      url = new URL(typeof input === "string" ? input : input.url, document.baseURI);
    } catch (_) {
      return realFetch(input, init);
    }
    const get = !init?.method || init.method === "GET";
    if (!get || !WASM.test(url.pathname)) return realFetch(input, init);
    let cache = null;
    try {
      cache = await caches.open(CACHE);
      const hit = await cache.match(url.href);
      if (hit) return hit;
    } catch (_) {
      // Cache Storage unavailable (private mode, quota): just download
    }
    const response = await realFetch(input, init);
    if (cache && response.ok && response.status === 200) {
      cache
        .put(url.href, response.clone())
        .then(async () => {
          // one app version at a time
          for (const old of await cache.keys()) if (WASM.test(new URL(old.url).pathname) && old.url !== url.href) await cache.delete(old);
        })
        .catch(() => {});
    }
    return response;
  };
}

export default function initializer() {
  const loading = () => document.getElementById("vectorcraft_loading");
  let bar = null;
  let label = null;
  // Bytes the browser will deliver after decompression: written into the page at build time by
  // `cargo xtask web-loading` (Content-Length is the compressed size, so it can't be used).
  const expected = () => Number(loading()?.dataset.wasmBytes) || 0;

  return {
    onStart() {
      try {
        if ("serviceWorker" in navigator) navigator.serviceWorker.register("./sw.js").catch(() => {});
      } catch (_) {
        // no service worker (private mode, insecure origin): the app still loads, just without the cache
      }
      const el = loading();
      if (!el) return;
      el.querySelector("[data-text]")?.replaceChildren(document.createTextNode("Loading ScreenFoundry…"));
      bar = el.querySelector("progress");
      label = el.querySelector("[data-mb]");
    },
    onProgress({ current, total }) {
      const all = expected() || total || 0;
      if (bar && all > 0) {
        bar.max = all;
        bar.value = Math.min(current, all);
      }
      if (label) label.textContent = all > 0 ? `${mb(current)} of ${mb(all)} MB` : `${mb(current)} MB`;
    },
    onComplete() {
      if (bar) bar.removeAttribute("value"); // compiling and starting: no number to show
      if (label) label.textContent = "Starting…";
    },
    onFailure(error) {
      console.error("ScreenFoundry could not load", error);
      if (label) label.textContent = "Could not load ScreenFoundry. Check your connection and reload.";
    },
  };
}
