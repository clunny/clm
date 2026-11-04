// The service worker served at `/sw.js` (web/retire-root-service-worker). The app's first
// service worker was registered at scope `/`, over the whole site. Now that the app lives at
// `/app/`, a browser that installed it still fetches `/sw.js` to check for an update; this
// is what it finds: a worker that takes over, drops the shell caches the old one made,
// unregisters itself, and hands the open pages back to the network. The new app's own shell
// cache holds `/app/` and the API copies are the app's either way, so both are left.

self.addEventListener('install', () => self.skipWaiting());

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      for (const key of await caches.keys()) {
        if (key.startsWith('clm-shell-') && !(await (await caches.open(key)).match('/app/'))) await caches.delete(key);
      }
      await self.registration.unregister();
      for (const client of await self.clients.matchAll({ type: 'window' })) {
        // Reload, so the page is no longer the old worker's.
        client.navigate(client.url).catch(() => undefined);
      }
    })(),
  );
});
