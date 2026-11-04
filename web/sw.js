// The service worker (web/installable), registered at `/app/sw.js` with scope `/app/`, so it
// never sees, answers or caches a request outside the app. `vite.config.ts` emits it, filling in
// the build's file list. It precaches the app shell and serves it offline. It never
// handles `/api/*` or `/auth/*`, which lie outside its scope: the app keeps its own API
// copies, with their age, and sign-in must always reach the Worker. It shows Web Push messages with the
// handlers in `src/live/push-sw.ts` (web/push-shown), whose built code the build puts in place of
// the import below, so the emitted worker is a classic script every browser installs.

import { onNotificationClick, onPush } from './sw-push.js';

const VERSION = self.__CLM_VERSION__;
const SHELL = self.__CLM_PRECACHE__;
const CACHE = `clm-shell-${VERSION}`;
// Servers send `Vary` (on `Origin`, say), and the precache's requests carry other headers
// than the page's; the URLs are content-hashed, so the URL alone decides.
const MATCH = { cacheName: CACHE, ignoreVary: true };

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((c) => c.addAll(SHELL))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith('clm-shell-') && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  const url = new URL(req.url);
  if (req.method !== 'GET' || url.origin !== self.location.origin) return;
  // Only the app's own files: the site beside it is the network's.
  if (!url.pathname.startsWith('/app/')) return;
  if (req.mode === 'navigate') {
    // `/app/`, not `/app/index.html`, which Workers assets redirect to `/app/`; a redirected
    // response can't answer a navigation.
    event.respondWith(fetch(req).catch(() => caches.match('/app/', MATCH)));
    return;
  }
  event.respondWith(caches.match(req, MATCH).then((hit) => hit ?? fetch(req)));
});

self.addEventListener('push', (event) => onPush(self, event));
self.addEventListener('notificationclick', (event) => onNotificationClick(self, event));
