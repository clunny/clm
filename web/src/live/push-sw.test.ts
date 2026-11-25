import { describe, expect, it, vi } from 'vitest';
import { onNotificationClick, onPush, type PushScope } from './push-sw';

interface FakeWindow {
  url: string;
  focus: ReturnType<typeof vi.fn<() => Promise<unknown>>>;
  navigate: ReturnType<typeof vi.fn<(url: string) => Promise<unknown>>>;
}

/** A fake service-worker `self` at `https://clm-app.dev`, with `windows` open. */
function scope(windows: FakeWindow[] = []) {
  const shown: { title: string; options: NotificationOptions }[] = [];
  const opened: string[] = [];
  const self: PushScope = {
    location: { origin: 'https://clm-app.dev' },
    registration: {
      showNotification: async (title, options = {}) => {
        shown.push({ title, options });
      },
    },
    clients: {
      matchAll: async () => windows,
      openWindow: async (url) => {
        opened.push(url);
        return null;
      },
    },
  };
  return { self, shown, opened };
}

/** An event whose `waitUntil` promises are collected, so the test can await them. */
function event<T extends object>(fields: T) {
  const waits: Promise<unknown>[] = [];
  return { e: { ...fields, waitUntil: (p: Promise<unknown>) => void waits.push(p) }, done: () => Promise.all(waits) };
}

function win(url: string): FakeWindow {
  const w: FakeWindow = { url, focus: vi.fn<() => Promise<unknown>>(), navigate: vi.fn<(url: string) => Promise<unknown>>() };
  w.focus.mockImplementation(async () => w);
  w.navigate.mockImplementation(async () => w);
  return w;
}

const PAYLOAD = { title: 'o/a: waiting on you', body: 'Which region?', url: '/app/#/o/a/backlog' };

describe('push messages', () => {
  // req:web.client-live/push-shown@9d6bf5a0
  it('notification click: shown with its title and body, tagged with its url, and a window opened there', async () => {
    const { self, shown, opened } = scope();
    const push = event({ data: { json: () => PAYLOAD } });
    onPush(self, push.e);
    await push.done();
    expect(shown).toHaveLength(1);
    expect(shown[0].title).toBe('o/a: waiting on you');
    expect(shown[0].options.body).toBe('Which region?');
    expect(shown[0].options.tag).toBe('/app/#/o/a/backlog');
    expect(shown[0].options.data).toEqual({ url: '/app/#/o/a/backlog' });
    expect(shown[0].options.icon).toBe('/app/icon-192.png');

    const close = vi.fn();
    const click = event({ notification: { data: shown[0].options.data, close } });
    onNotificationClick(self, click.e);
    await click.done();
    expect(close).toHaveBeenCalled();
    expect(opened).toEqual(['https://clm-app.dev/app/#/o/a/backlog']);
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a click with an app window open focuses it and navigates it to the url', async () => {
    const other = win('https://example.com/');
    const app = win('https://clm-app.dev/app/#/o/a/graph');
    const { self, opened } = scope([other, app]);
    const click = event({ notification: { data: { url: '/app/#/o/a/backlog' }, close: vi.fn() } });
    onNotificationClick(self, click.e);
    await click.done();
    expect(app.focus).toHaveBeenCalled();
    expect(app.navigate).toHaveBeenCalledWith('https://clm-app.dev/app/#/o/a/backlog');
    expect(other.focus).not.toHaveBeenCalled();
    expect(opened).toEqual([]);
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a push without a payload, or with one that is not JSON, still shows a notification', async () => {
    for (const data of [null, { json: () => JSON.parse('not json') }]) {
      const { self, shown } = scope();
      const push = event({ data });
      onPush(self, push.e);
      await push.done();
      expect(shown.map((s) => s.title)).toEqual(['CLM']);
    }
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a window the worker does not control, which navigate or focus rejects, gets a new window instead', async () => {
    for (const fails of ['navigate', 'focus'] as const) {
      const app = win('https://clm-app.dev/app/#/o/a/graph');
      app[fails].mockImplementation(async () => {
        throw new TypeError('not controlled');
      });
      const { self, opened } = scope([app]);
      const click = event({ notification: { data: { url: '/app/#/o/a/backlog' }, close: vi.fn() } });
      onNotificationClick(self, click.e);
      await click.done();
      expect(opened, fails).toEqual(['https://clm-app.dev/app/#/o/a/backlog']);
    }
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a site page is open: it is not navigated, and a window is opened at the url', async () => {
    const site = win('https://clm-app.dev/guide/quickstart');
    const { self, opened } = scope([site]);
    const click = event({ notification: { data: { url: '/app/#/o/a/backlog' }, close: vi.fn() } });
    onNotificationClick(self, click.e);
    await click.done();
    expect(site.navigate).not.toHaveBeenCalled();
    expect(opened).toEqual(['https://clm-app.dev/app/#/o/a/backlog']);
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a message with no url is tagged with the app root', async () => {
    const { self, shown } = scope();
    const push = event({ data: { json: () => ({ title: 't' }) } });
    onPush(self, push.e);
    await push.done();
    expect(shown[0].options.tag).toBe('/app/');
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a url on another origin, or one that does not parse, opens the app instead', async () => {
    const { self, opened } = scope();
    for (const url of ['https://evil.example/x', 'http://[']) {
      const click = event({ notification: { data: { url }, close: vi.fn() } });
      onNotificationClick(self, click.e);
      await click.done();
    }
    expect(opened).toEqual(['https://clm-app.dev/app/', 'https://clm-app.dev/app/']);
  });

  // req:web.client-live/push-shown@9d6bf5a0
  it('a payload of the wrong shape', async () => {
    const { self, shown } = scope();
    for (const p of ['x', { title: 5, body: 5, url: 5 }, { title: 't' }]) {
      const push = event({ data: { json: () => p } });
      onPush(self, push.e);
      await push.done();
    }
    expect(shown.map((s) => s.title)).toEqual(['CLM', 'CLM', 't']);
    expect(shown.map((s) => s.options.body)).toEqual(['', '', '']);
    expect(shown.map((s) => s.options.tag)).toEqual(['/app/', '/app/', '/app/']);
  });
});
