// The retry schedule and the words for each failure.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpError, Offline } from './views/api';
import { classify, failureText, retryDelay } from './errors';

describe('retryDelay', () => {
  // req:web/honest-errors@444ce2bc
  it('backs off 5, 10, 20, then every 60 seconds', () => {
    expect([0, 1, 2, 3, 4, 9].map(retryDelay)).toEqual([5000, 10000, 20000, 60000, 60000, 60000]);
  });
});

describe('failures', () => {
  beforeEach(() => vi.stubGlobal('navigator', { onLine: true }));
  afterEach(() => vi.unstubAllGlobals());

  // req:web/honest-errors@444ce2bc
  it('a 404 is about access, a preparing 503 gives its share, other failures retry', () => {
    expect(classify(new HttpError('/x', 404))).toEqual({ kind: 'no-access' });
    expect(classify(new HttpError('/x', 403))).toEqual({ kind: 'no-access' });
    expect(classify(new HttpError('/x', 503, 0.4))).toEqual({ kind: 'preparing', share: 0.4 });
    expect(classify(new HttpError('/x', 503))).toEqual({ kind: 'retry' });
    expect(classify(new HttpError('/x', 502))).toEqual({ kind: 'retry' });
    expect(classify(new Offline())).toEqual({ kind: 'retry' });
    expect(classify(new Offline())).toEqual({ kind: 'retry' });
  });

  // req:web/honest-errors@444ce2bc
  it('retry text says nothing about access', () => {
    expect(failureText({ kind: 'retry' }).text).toBe("CLM couldn't load the repo just now and is retrying.");
    expect(failureText({ kind: 'retry' }).text).not.toMatch(/account|App/);
    expect(failureText({ kind: 'preparing', share: 0.4 }).text).toContain('40% done');
    expect(failureText({ kind: 'no-access' }).text).toContain("isn't readable with your GitHub account");
  });
});
