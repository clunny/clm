// The Worker's API, mocked for browser tests.

import type { BrowserContext } from '@playwright/test';
import { architecture, branches, capabilities, implementation, review, snapshot } from '../src/views/fixtures';
import type { Review } from '../src/views/types';

/** `spec/c`'s review and approve routes. */
export const REVIEW = 'GET /api/repos/o/a/branches/spec/c/review';
export const APPROVE = 'POST /api/repos/o/a/branches/spec/c/approve';
/** `spec/c`'s comment and request-changes routes, and the backlog's proposals. */
export const COMMENTS = 'POST /api/repos/o/a/branches/spec/c/comments';
export const REQUEST = 'POST /api/repos/o/a/branches/spec/c/request-changes';
export const PROPOSALS = 'POST /api/repos/o/a/proposals';
/** The capabilities report, and the decide route of capability `a`. */
export const CAPABILITIES = 'GET /api/repos/o/a/capabilities';
export const DECIDE = 'POST /api/repos/o/a/capabilities/a/decide';
/** The architecture report, and the decide route of its entry `e`. */
export const ARCHITECTURE = 'GET /api/repos/o/a/architecture';
export const ARCH_DECIDE = 'POST /api/repos/o/a/architecture/e/decide';

export interface Mock {
  /** The writes (posts and deletes) the app made, as `METHOD /path`, with their bodies. */
  posts: { key: string; body: string | null }[];
  /** The responses by `METHOD /path`; change one to change what the Worker answers next. A
   * post answers 200 unless `fail` says otherwise. */
  routes: Record<string, unknown>;
  /** Every API request the app made, as `METHOD /path`, in order. */
  requests: string[];
  /** The same, with each query string: `METHOD /path?query`. */
  urls: string[];
  /** Holds the answers to `key`, or to everything, until `release`. */
  hold(key?: string): void;
  release(key?: string): void;
  /** Answers `key` with `status` and no body, or `body` as JSON when given, or fails the
   * request as a network does (`'abort'`); `null` answers normally again. */
  fail(key: string, status: number | 'abort' | null, body?: unknown): void;
  /** Called after a post is answered, to change what the Worker answers next. */
  onPost?: (key: string) => void;
}

export async function mockApi(context: BrowserContext, pr: Review = review()): Promise<Mock> {
  const held = new Map<string, (() => void)[]>();
  const failing = new Map<string, number | 'abort'>();
  const failBodies = new Map<string, string>();
  const failWith = (status: number, key: string) => {
    const body = failBodies.get(key);
    return body === undefined ? { status, body: '' } : { status, contentType: 'application/json', body };
  };
  const ALL = '*';
  const mock: Mock = {
    posts: [],
    routes: {
      'GET /api/repos': [{ owner: 'o', name: 'a', default_branch: 'main' }],
      'GET /api/repos/o/a/snapshot': snapshot(),
      'GET /api/repos/o/a/branches': branches,
      'GET /api/repos/o/a/implementation': implementation(),
      [REVIEW]: pr,
      [CAPABILITIES]: capabilities(),
      [ARCHITECTURE]: architecture(),
    },
    requests: [],
    urls: [],
    hold: (key = ALL) => held.set(key, held.get(key) ?? []),
    release: (key = ALL) => {
      for (const go of held.get(key) ?? []) go();
      held.delete(key);
    },
    fail: (key, status, body) => {
      if (status === null) failing.delete(key);
      else failing.set(key, status);
      if (body === undefined) failBodies.delete(key);
      else failBodies.set(key, JSON.stringify(body));
    },
  };
  await context.route('**/api/**', async (route) => {
    const req = route.request();
    const key = `${req.method()} ${new URL(req.url()).pathname}`;
    mock.requests.push(key);
    mock.urls.push(`${key}${new URL(req.url()).search}`);
    const waiting = held.get(key) ?? held.get(ALL);
    if (waiting) await new Promise<void>((go) => waiting.push(go));
    const status = failing.get(key);
    if (status === 'abort') return route.abort('failed');
    if (req.method() === 'POST' || req.method() === 'DELETE') {
      mock.posts.push({ key, body: req.postData() });
      mock.onPost?.(key);
      return route.fulfill(failWith(status ?? 200, key));
    }
    if (status !== undefined) return route.fulfill(failWith(status, key));
    const body = mock.routes[key];
    return body === undefined
      ? route.fulfill({ status: 404, body: '' })
      : route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
  });
  return mock;
}

/** A review touching node `b` of the fixture snapshot. */
export function reviewOfB(): Review {
  const r = review();
  r.nodes = [{ ...r.nodes[0], id: 'b' }];
  return r;
}
