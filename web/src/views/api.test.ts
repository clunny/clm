// The views' one module for requests to the Worker: offline data with its age, and the
// retry on 401 that the Worker's one-use refresh tokens need.
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
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

const ok = (body: unknown) => new Response(JSON.stringify(body), { status: 200 });

describe('api.get', () => {
  // req:web.views/worker-requests@1a1729dd
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

  // req:web.views/worker-requests@1a1729dd
  it('is signed out after two 401s, and forgets the kept data', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 1 })));
    await get('/api/repos/o/a/snapshot');
    expect(caches.store.size).toBe(1);
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 401 })));
    await expect(get('/api/repos/o/a/snapshot')).rejects.toBeInstanceOf(SignedOut);
    expect(caches.store.size).toBe(0);
  });

  // req:web.views/worker-requests@1a1729dd
  it('offline: returns the last response with the time it was fetched', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 'old' })));
    vi.setSystemTime(new Date(1_000_000));
    await get('/api/repos/o/a/snapshot');
    vi.useRealTimers();
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
    const res = await get<{ v: string }>('/api/repos/o/a/snapshot');
    expect(res).toEqual({ data: { v: 'old' }, cachedAt: 1_000_000 });
  });

  // req:web.views/worker-requests@1a1729dd
  it('offline with nothing kept: says so', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
    await expect(get('/api/repos/o/b/snapshot')).rejects.toBeInstanceOf(Offline);
  });

  // req:web.views/worker-requests@1a1729dd
  it('keeps no failed response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('', { status: 404 })));
    const failed = await get('/api/repos/o/c/snapshot').catch((e: unknown) => e);
    expect(failed).toBeInstanceOf(HttpError);
    expect((failed as HttpError).status).toBe(404);
    expect(caches.store.size).toBe(0);
  });

  // req:web.views/worker-requests@1a1729dd
  it('keeps no answer its caller asked not to keep', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 1 })));
    expect(await get('/api/repos/o/a/push?endpoint=e', { keep: false })).toEqual({ data: { v: 1 }, cachedAt: null });
    expect(caches.store.size).toBe(0);
  });

  // req:web.views/worker-requests@1a1729dd
  it('a background refresh fails offline rather than return the kept answer', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 'old' })));
    await get('/api/repos/o/a/snapshot');
    const down = new TypeError('Failed to fetch');
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(down));
    await expect(get('/api/repos/o/a/snapshot', { live: true })).rejects.toBe(down);
  });
});

describe('api.post', () => {
  // req:web.views/worker-requests@1a1729dd
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

describe('a refresh a notice asked for', () => {
  // req:web.views/worker-requests@1a1729dd
  it('carries fresh, and keeps the offline copy under the path without it', async () => {
    const fetch = vi.fn().mockImplementation(async () => ok({ v: 1 }));
    vi.stubGlobal('fetch', fetch);
    await get('/api/repos/o/a/implementation', { live: true, fresh: 'd1' });
    await get('/api/repos/o/a/branches/spec/x/review?x=1', { live: true, fresh: 'd1' });
    expect(fetch.mock.calls.map(([p]) => p)).toEqual(['/api/repos/o/a/implementation?fresh=d1', '/api/repos/o/a/branches/spec/x/review?x=1&fresh=d1']);
    expect([...caches.store.keys()].sort()).toEqual(['/api/repos/o/a/branches/spec/x/review?x=1', '/api/repos/o/a/implementation']);
  });
});

describe('api.send', () => {
  // req:web.views/worker-requests@1a1729dd
  it('a post answered 401 is sent once more, and a second 401 reports the person signed out', async () => {
    const fetch = vi.fn().mockResolvedValueOnce(new Response('', { status: 401 })).mockResolvedValueOnce(new Response('{}', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    expect((await post('/api/repos/o/a/branches/spec/x/approve', { fingerprint: 'f1' })).status).toBe(200);
    expect(fetch).toHaveBeenCalledTimes(2);
    const denied = vi.fn().mockResolvedValue(new Response('', { status: 401 }));
    vi.stubGlobal('fetch', denied);
    await expect(send('DELETE', '/api/repos/o/a/push', { endpoint: 'e' })).rejects.toBeInstanceOf(SignedOut);
    expect(denied).toHaveBeenCalledTimes(2);
  });

  // req:web.views/worker-requests@1a1729dd
  it('deletes with a JSON body, and a failed get carries its status', async () => {
    const fetch = vi.fn().mockResolvedValueOnce(new Response('', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    await send('DELETE', '/api/repos/o/a/push', { endpoint: 'e' });
    const [path, init] = fetch.mock.calls[0];
    expect(path).toBe('/api/repos/o/a/push');
    expect(init).toMatchObject({ method: 'DELETE', body: '{"endpoint":"e"}', headers: { 'content-type': 'application/json' } });
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('', { status: 404 })));
    const failed = await get('/api/push/key').catch((e: unknown) => e);
    expect(failed).toBeInstanceOf(HttpError);
    expect((failed as HttpError).status).toBe(404);
  });
});

describe('api.get on a 5xx', () => {
  // req:web.views/worker-requests@1a1729dd
  it('falls back to the kept copy, but not for a live refresh or a preparing 503', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('{"v":1}', { status: 200 })));
    await get('/p');
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('{"error":"unavailable"}', { status: 502 })));
    expect((await get<{ v: number }>('/p')).data).toEqual({ v: 1 });
    const background = get('/p', { live: true });
    await expect(background).rejects.toBeInstanceOf(HttpError);
    await expect(background).rejects.toMatchObject({ status: 502 });
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('{"preparing":0.4}', { status: 503 })));
    await expect(get('/p')).rejects.toMatchObject({ status: 503, preparing: 0.4 });
  });

  // req:web.views/worker-requests@1a1729dd
  it('busy Worker: a 502 returns the kept answer with its time, a background refresh fails with 502, a preparing 503 gives its share, and fresh joins an existing query', async () => {
    vi.setSystemTime(new Date(2_000_000));
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(ok({ v: 1 })));
    await get('/p');
    vi.useRealTimers();
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('', { status: 502 })));
    expect(await get('/p')).toEqual({ data: { v: 1 }, cachedAt: 2_000_000 });
    await expect(get('/p', { live: true })).rejects.toMatchObject({ status: 502, preparing: null });
    // With nothing kept, a 502 fails carrying its status.
    await expect(get('/none')).rejects.toMatchObject({ status: 502 });
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('{"preparing":0.4}', { status: 503 })));
    await expect(get('/p')).rejects.toMatchObject({ status: 503, preparing: 0.4 });
    const fetch = vi.fn().mockImplementation(async () => ok({ q: 1 }));
    vi.stubGlobal('fetch', fetch);
    await get('/q?x=1', { fresh: 'd1' });
    expect(fetch.mock.calls[0][0]).toBe('/q?x=1&fresh=d1');
    expect([...caches.store.keys()]).toContain('/q?x=1');
  });
});
