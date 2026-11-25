// What the app says when a request for a repo's view fails (web/honest-errors): never more
// than it knows.

import { HttpError } from './views/api';

/** The seconds between asks after a failure: 5, 10, 20, then every 60. */
const BACKOFF_S = [5, 10, 20, 60];

/** The wait before the `attempt`th ask again (from 0), in ms. */
export const retryDelay = (attempt: number): number => BACKOFF_S[Math.min(attempt, BACKOFF_S.length - 1)] * 1000;

/** A preparing repo is asked again at this interval. */
export const PREPARING_MS = 5000;

/** A view whose refreshes have failed this long shows how old it is. */
export const STALE_MS = 180_000;

export type Failure =
  /** A 403 or a 404: the repo isn't readable, or the App doesn't cover it. */
  | { kind: 'no-access' }
  /** Any other 4xx but a 401: the request failed, and nothing more is known. */
  | { kind: 'failed'; status: number }
  | { kind: 'preparing'; share: number }
  /** CLM is busy or unreachable: ask again. */
  | { kind: 'retry' }
  /** The Worker answered, but not with what the view is drawn from. */
  | { kind: 'undrawable' }
  ;

/** A view that couldn't be drawn from what the Worker answered, with why. */
export class Undrawable extends Error {
  constructor(readonly why: unknown) {
    super(`couldn't be drawn: ${why instanceof Error ? why.message : String(why)}`);
  }
}

export function classify(e: unknown): Failure {
  // A body that isn't JSON, or a view that threw while drawing from it.
  if (e instanceof Undrawable || e instanceof SyntaxError) return { kind: 'undrawable' };
  // A network failure, whether before the answer or while reading it, is asked again.
  if (!(e instanceof HttpError)) return { kind: 'retry' };
  if (e.status === 403 || e.status === 404) return { kind: 'no-access' };
  if (e.status < 500) return { kind: 'failed', status: e.status };
  if (e.preparing !== null) return { kind: 'preparing', share: e.preparing };
  return { kind: 'retry' };
}

/** The heading and the text for a failure. */
export function failureText(f: Failure): { title: string; text: string } {
  switch (f.kind) {
    case 'no-access':
      return { title: 'Not available', text: "This repo isn't readable with your GitHub account, or isn't covered by the App." };
    case 'failed':
      return { title: 'Not loaded', text: `The request failed (${f.status}).` };
    case 'preparing':
      return { title: 'Preparing the repo', text: `CLM is preparing this repo: ${Math.round(Math.min(Math.max(f.share, 0), 1) * 100)}% done.` };
    case 'retry':
      return { title: 'Not loaded yet', text: "CLM couldn't load the repo just now and is retrying." };
    case 'undrawable':
      return { title: 'Not drawn', text: "This view couldn't be drawn from what CLM answered." };
  }
}
