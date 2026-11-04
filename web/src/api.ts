// The Worker's API. The last good response for each URL is kept in Cache Storage, so the
// app can show it, with its age, when it's offline (web/installable).

const CACHE = 'clm-api';
const AT = 'x-clm-at';

export class SignedOut extends Error {
  constructor() {
    super('signed out');
  }
}

export class Offline extends Error {
  constructor() {
    super('offline, and nothing kept for this view');
  }
}

/** A response other than 2xx or 401. */
export class HttpError extends Error {
  constructor(
    path: string,
    readonly status: number,
    /** The share done of a 503 `{"preparing": <share>}`: the Worker is preparing the repo. */
    readonly preparing: number | null = null,
  ) {
    super(`${path}: ${status}`);
  }
}

export interface Fetched<T> {
  data: T;
  /** When the data was fetched, in ms, if it came from the kept copy; null when live. */
  cachedAt: number | null;
}

async function request(path: string, init?: RequestInit): Promise<Response> {
  const go = () => fetch(path, { credentials: 'same-origin', ...init });
  const res = await go();
  // GitHub refresh tokens work once: when parallel requests carried an expired session,
  // one refreshed it and re-set the cookie, and the others got 401. Try again once.
  return res.status === 401 ? go() : res;
}

export interface GetOptions {
  /** A background refresh (web/live-refresh): when the network fails, fail rather than fall
   * back to the kept copy, so the view on screen is left as it is. */
  live?: boolean;
  /** The delivery of the live notice that asked for this refresh (web/live-updates): sent as
   * `?fresh=`, so the Worker answers from live state fetched anew. The copy kept for offline
   * use stays under `path` without it. */
  fresh?: string;
  /** False for an answer never shown offline, such as a push subscription's state: it isn't
   * kept, so its endpoint isn't stored either. */
  keep?: boolean;
}

/** The share in a 503 `{"preparing": <share>}`, or null for any other body. */
async function preparingShare(res: Response): Promise<number | null> {
  const body: unknown = await res.json().catch(() => null);
  const share = typeof body === 'object' && body !== null ? (body as { preparing?: unknown }).preparing : undefined;
  return typeof share === 'number' ? share : null;
}

export async function get<T>(path: string, o: GetOptions = {}): Promise<Fetched<T>> {
  // The copy kept for offline use, for a network failure or a 5xx that isn't "preparing"; a
  // background refresh has none, so it fails and leaves the view as it is.
  const kept = async (): Promise<Fetched<T> | null> => {
    if (o.live) return null;
    const copy = await caches
      .open(CACHE)
      .then((c) => c.match(path))
      .catch(() => undefined);
    return copy ? { data: (await copy.json()) as T, cachedAt: Number(copy.headers.get(AT)) } : null;
  };
  let res: Response;
  try {
    res = await request(o.fresh === undefined ? path : `${path}${path.includes('?') ? '&' : '?'}fresh=${encodeURIComponent(o.fresh)}`);
  } catch (e) {
    if (o.live) throw e;
    const copy = await kept();
    if (!copy) throw new Offline();
    return copy;
  }
  if (res.status === 401) {
    // Whoever signs in next on this browser mustn't see the last person's data.
    await caches.delete(CACHE).catch(() => undefined);
    throw new SignedOut();
  }
  if (!res.ok) {
    const share = res.status === 503 ? await preparingShare(res) : null;
    if (res.status >= 500 && share === null) {
      const copy = await kept();
      if (copy) return copy;
    }
    throw new HttpError(path, res.status, share);
  }
  const text = await res.text();
  if (o.keep !== false) {
    const copy = new Response(text, { headers: { 'content-type': 'application/json', [AT]: String(Date.now()) } });
    await caches
      .open(CACHE)
      .then((c) => c.put(path, copy))
      .catch(() => undefined);
  }
  return { data: JSON.parse(text) as T, cachedAt: null };
}

/** Sends `method` to `path`, with `body` as JSON when given. */
export async function send(method: 'POST' | 'DELETE', path: string, body?: unknown): Promise<Response> {
  const init: RequestInit = { method };
  if (body !== undefined) Object.assign(init, { body: JSON.stringify(body), headers: { 'content-type': 'application/json' } });
  const res = await request(path, init);
  if (res.status === 401) throw new SignedOut();
  return res;
}

/** Posts to `path`, with `body` as JSON when given. */
export function post(path: string, body?: unknown): Promise<Response> {
  return send('POST', path, body);
}

/** The Worker's path of repo `owner/repo`. */
export const apiBase = (owner: string, repo: string) => `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}`;
