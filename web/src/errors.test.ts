// req:web/honest-errors@444ce2bc: the retry schedule, the words for each failure, and what the
// API client does with a 5xx.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpError, Offline, get } from './api';
import { classify, failureText, retryDelay } from './errors';

describe('retryDelay', () => {
  it('backs off 5, 10, 20, then every 60 seconds', () => {
    expect([0, 1, 2, 3, 4, 9].map(retryDelay)).toEqual([5000, 10000, 20000, 60000, 60000, 60000]);
  });
});

describe('failures', () => {
  beforeEach(() => vi.stubGlobal('navigator', { onLine: true }));
  afterEach(() => vi.unstubAllGlobals());

  it('a 404 is about access, a preparing 503 gives its share, other failures retry', () => {
    expect(classify(new HttpError('/x', 404))).toEqual({ kind: 'no-access' });
    expect(classify(new HttpError('/x', 403))).toEqual({ kind: 'no-access' });
    expect(classify(new HttpError('/x', 503, 0.4))).toEqual({ kind: 'preparing', share: 0.4 });
    expect(classify(new HttpError('/x', 503))).toEqual({ kind: 'retry' });
    expect(classify(new HttpError('/x', 502))).toEqual({ kind: 'retry' });
    expect(classify(new Offline())).toEqual({ kind: 'retry' });
    expect(classify(new Offline())).toEqual({ kind: 'retry' });
  });

  it('retry text says nothing about access', () => {
    expect(failureText({ kind: 'retry' }).text).toBe("CLM couldn't load the repo just now and is retrying.");
    expect(failureText({ kind: 'retry' }).text).not.toMatch(/account|App/);
    expect(failureText({ kind: 'preparing', share: 0.4 }).text).toContain('40% done');
    expect(failureText({ kind: 'no-access' }).text).toContain("isn't readable with your GitHub account");
  });
});

describe('api.get on a 5xx', () => {
  const store = new Map<string, Response>();
  beforeEach(() => {
    store.clear();
    const cache = { match: async (k: string) => store.get(k)?.clone(), put: async (k: string, r: Response) => void store.set(k, r) };
    vi.stubGlobal('caches', { open: async () => cache, delete: async () => true });
  });
  afterEach(() => vi.unstubAllGlobals());

  it('falls back to the kept copy, but not for a live refresh or a preparing 503', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('{"v":1}', { status: 200 })));
    await get('/p');
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('{"error":"unavailable"}', { status: 502 })));
    expect((await get<{ v: number }>('/p')).data).toEqual({ v: 1 });
    await expect(get('/p', { live: true })).rejects.toBeInstanceOf(HttpError);
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async () => new Response('{"preparing":0.4}', { status: 503 })));
    await expect(get('/p')).rejects.toMatchObject({ status: 503, preparing: 0.4 });
  });
});
