// The retry schedule and the words for each failure.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpError, Offline } from './views/api';
import { Undrawable, classify, failureText, retryDelay } from './errors';

describe('retryDelay', () => {
  // req:web/honest-errors@165e071b
  it('backs off 5, 10, 20, then every 60 seconds', () => {
    expect([0, 1, 2, 3, 4, 9].map(retryDelay)).toEqual([5000, 10000, 20000, 60000, 60000, 60000]);
  });
});

describe('failures', () => {
  beforeEach(() => vi.stubGlobal('navigator', { onLine: true }));
  afterEach(() => vi.unstubAllGlobals());

  // req:web/honest-errors@165e071b
  it('no access: a 403 or a 404 is about access', () => {
    expect(classify(new HttpError('/x', 404))).toEqual({ kind: 'no-access' });
    expect(classify(new HttpError('/x', 403))).toEqual({ kind: 'no-access' });
    expect(failureText({ kind: 'no-access' }).text).toBe("This repo isn't readable with your GitHub account, or isn't covered by the App.");
  });

  // req:web/honest-errors@165e071b
  it('other refusals: any other 4xx says the request failed, with its status, and nothing about access', () => {
    expect(classify(new HttpError('/x', 429))).toEqual({ kind: 'failed', status: 429 });
    expect(classify(new HttpError('/x', 400))).toEqual({ kind: 'failed', status: 400 });
    const { text } = failureText(classify(new HttpError('/x', 429)));
    expect(text).toBe('The request failed (429).');
    expect(text).not.toMatch(/account|App|access/);
  });

  // req:web/honest-errors@165e071b
  it('a preparing 503 gives its share, held between 0 and 100 %, and other 5xx and the network retry', () => {
    expect(classify(new HttpError('/x', 503, 0.4))).toEqual({ kind: 'preparing', share: 0.4 });
    expect(failureText({ kind: 'preparing', share: 0.4 }).text).toContain('40% done');
    expect(failureText(classify(new HttpError('/x', 503, 1.7))).text).toContain('100% done');
    expect(failureText(classify(new HttpError('/x', 503, -0.2))).text).toContain('0% done');
    expect(classify(new HttpError('/x', 503))).toEqual({ kind: 'retry' });
    expect(classify(new HttpError('/x', 502))).toEqual({ kind: 'retry' });
    expect(classify(new Offline())).toEqual({ kind: 'retry' });
  });

  // req:web/honest-errors@165e071b
  it('retry text says nothing about access', () => {
    expect(failureText({ kind: 'retry' }).text).toBe("CLM couldn't load the repo just now and is retrying.");
    expect(failureText({ kind: 'retry' }).text).not.toMatch(/account|App/);
  });

  // req:web/honest-errors@165e071b
  it("a view that can't be drawn from what the Worker answered says so, and not that CLM is retrying", () => {
    expect(classify(new Undrawable(new TypeError("Cannot read properties of undefined (reading 'nodes')")))).toEqual({ kind: 'undrawable' });
    expect(classify(new SyntaxError('Unexpected token'))).toEqual({ kind: 'undrawable' });
    // A network failure while the body is read is no answer to draw from: it is asked again.
    expect(classify(new TypeError('network error'))).toEqual({ kind: 'retry' });
    const { text } = failureText({ kind: 'undrawable' });
    expect(text).toBe("This view couldn't be drawn from what CLM answered.");
    expect(text).not.toMatch(/retrying/);
  });
});
