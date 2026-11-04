// The service worker's Web Push handlers (web.client-live/push-shown): a push shows its payload's
// `title` and `body`, tagged with its `url` so a repeat replaces it, and a click focuses
// an open app window and navigates it to the `url`, or opens one there. `sw.js` imports
// this module and passes it the real `self`; the tests pass a fake one.

const APP = '/app/';

/** What the handlers use of a service worker's global scope. */
export interface PushScope {
  location: { origin: string };
  registration: { showNotification(title: string, options?: NotificationOptions): Promise<void> };
  clients: {
    matchAll(options: { type: 'window'; includeUncontrolled: boolean }): Promise<readonly WindowLike[]>;
    openWindow(url: string): Promise<unknown>;
  };
}

interface WindowLike {
  url: string;
  focus(): Promise<unknown>;
  navigate(url: string): Promise<unknown>;
}

interface Waits {
  waitUntil(p: Promise<unknown>): void;
}

/** The Worker's notice: `web.live`'s `waiting_notice` and `review_notice`. */
interface Payload {
  title?: unknown;
  body?: unknown;
  url?: unknown;
}

function payload(data: { json(): unknown } | null | undefined): Payload {
  try {
    const p = data?.json();
    return p && typeof p === 'object' ? (p as Payload) : {};
  } catch {
    return {};
  }
}

/** `url` on the app's own origin, or the app's root, `/app/`, for anything else. */
function inApp(self: PushScope, url: unknown): string {
  const root = `${self.location.origin}${APP}`;
  if (typeof url !== 'string') return root;
  try {
    const u = new URL(url, root);
    return u.origin === self.location.origin ? u.href : root;
  } catch {
    return root;
  }
}

export function onPush(self: PushScope, event: Waits & { data?: { json(): unknown } | null }): void {
  const p = payload(event.data);
  const url = typeof p.url === 'string' ? p.url : APP;
  const options: NotificationOptions = { body: typeof p.body === 'string' ? p.body : '', tag: url, data: { url }, icon: `${APP}icon-192.png` };
  event.waitUntil(self.registration.showNotification(typeof p.title === 'string' ? p.title : 'CLM', options));
}

export function onNotificationClick(self: PushScope, event: Waits & { notification: { data?: unknown; close(): void } }): void {
  event.notification.close();
  const data = event.notification.data as { url?: unknown } | null | undefined;
  const url = inApp(self, data?.url);
  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then(async (windows) => {
      const open = windows.find((w) => w.url.startsWith(`${self.location.origin}${APP}`));
      if (!open) return self.clients.openWindow(url);
      await open.focus();
      // `navigate` rejects for a window this worker doesn't control (one hard-reloaded, or
      // opened before it took over): that one opens a window at the url instead.
      return open.navigate(url).catch(() => self.clients.openWindow(url));
    }),
  );
}
