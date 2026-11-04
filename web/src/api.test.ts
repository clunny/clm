// req:web/installable@e711892d (offline data with its age) and the retry on 401 that the Worker's
// one-use refresh tokens need (changes/20261022-web-ui/tasks/web.md).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpError, Offline, SignedOut, get, post, send } from './api';

/** A Cache Storage stand-in: one cache, keyed by URL. */
function fakeCaches() {
  const store = new Map<string, Response>();
  const cache = {
    async match(req: string) {
      return store.get(req)?.clone();
    },
    async put(req: string, res: Response) {
      store.set(req, res);
    },
  };
  return {
    open: async () => cache,
    delete: async () => {
      store.clear();
      return true;
    },
    store,
  };
}

let caches: ReturnType<typeof fakeCaches>;

beforeEach(() => {
  caches = fakeCaches();
  vi.stubGlobal('caches', caches);
});
afterEach(() => vi.unstubAllGlobals());

const ok = (body: unknown) => new Response(JSON.stringify(body), { status: 200 });

describe('api.get', () => {
  it('retries once on 401, since a parallel request may have refreshed the session', async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response('', { status: 401 }))
      .mockResolvedValueOnce(ok({ a: 1 }));
    vi.stubGlobal('fetch', fetch);
    const res = await get<{ a: number }>('/api/repos');
    expect(res).toEqual({ data: { a: 1 }, cachedAt: null });
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('is signed out after two 401s, and forgets the kept data', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 1 })));
    await get('/api/repos/o/a/snapshot');
    expect(caches.store.size).toBe(1);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 401 })));
    await expect(get('/api/repos')).rejects.toBeInstanceOf(SignedOut);
    expect(caches.store.size).toBe(0);
  });

  it('offline: returns the last response with the time it was fetched', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 'old' })));
    vi.setSystemTime(new Date(1_000_000));
    await get('/api/repos/o/a/snapshot');
    vi.useRealTimers();
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
    const res = await get<{ v: string }>('/api/repos/o/a/snapshot');
    expect(res).toEqual({ data: { v: 'old' }, cachedAt: 1_000_000 });
  });

  it('offline with nothing kept: says so', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
    await expect(get('/api/repos/o/b/snapshot')).rejects.toBeInstanceOf(Offline);
  });

  it('keeps no failed response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('', { status: 404 })));
    await expect(get('/api/repos/o/c/snapshot')).rejects.toThrow('404');
    expect(caches.store.size).toBe(0);
  });
});

// req:web/approve-by-branch@971e758a
describe('api.post', () => {
  it('sends a body as JSON', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    await post('/api/repos/o/a/branches/spec/x/approve', { fingerprint: 'f1' });
    const [path, init] = fetch.mock.calls[0];
    expect(path).toBe('/api/repos/o/a/branches/spec/x/approve');
    expect(init.method).toBe('POST');
    expect(init.body).toBe('{"fingerprint":"f1"}');
    expect(init.headers).toEqual({ 'content-type': 'application/json' });
  });
});

// req:web/notice-refresh@4f65ff27
describe('a refresh a notice asked for', () => {
  it('carries fresh, and keeps the offline copy under the path without it', async () => {
    const fetch = vi.fn().mockImplementation(async () => ok({ v: 1 }));
    vi.stubGlobal('fetch', fetch);
    await get('/api/repos/o/a/implementation', { live: true, fresh: 'd1' });
    await get('/api/repos/o/a/push?endpoint=e', { live: true, fresh: 'd1' });
    expect(fetch.mock.calls.map(([p]) => p)).toEqual(['/api/repos/o/a/implementation?fresh=d1', '/api/repos/o/a/push?endpoint=e&fresh=d1']);
    expect([...caches.store.keys()].sort()).toEqual(['/api/repos/o/a/implementation', '/api/repos/o/a/push?endpoint=e']);
  });
});

// req:web.client-live/notify-switch@59a9bf95
describe('api.send', () => {
  it('deletes with a JSON body, and a failed get carries its status', async () => {
    const fetch = vi.fn().mockResolvedValueOnce(new Response('', { status: 200 })).mockResolvedValueOnce(new Response('', { status: 404 }));
    vi.stubGlobal('fetch', fetch);
    await send('DELETE', '/api/repos/o/a/push', { endpoint: 'e' });
    expect(fetch.mock.calls[0][1]).toMatchObject({ method: 'DELETE', body: '{"endpoint":"e"}' });
    const failed = await get('/api/push/key').catch((e: unknown) => e);
    expect(failed).toBeInstanceOf(HttpError);
    expect((failed as HttpError).status).toBe(404);
  });
});
