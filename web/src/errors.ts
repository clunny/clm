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
  | { kind: 'no-access' }
  | { kind: 'preparing'; share: number }
  /** CLM is busy or unreachable: ask again. */
  | { kind: 'retry' }
  ;

export function classify(e: unknown): Failure {
  if (e instanceof HttpError && e.status < 500) return { kind: 'no-access' };
  if (e instanceof HttpError && e.preparing !== null) return { kind: 'preparing', share: e.preparing };
  return { kind: 'retry' };
}

/** The heading and the text for a failure. */
export function failureText(f: Failure): { title: string; text: string } {
  switch (f.kind) {
    case 'no-access':
      return { title: 'Not available', text: "This repo isn't readable with your GitHub account, or isn't covered by the App." };
    case 'preparing':
      return { title: 'Preparing the repo', text: `CLM is preparing this repo: ${Math.round(Math.min(Math.max(f.share, 0), 1) * 100)}% done.` };
    case 'retry':
      return { title: 'Not loaded yet', text: "CLM couldn't load the repo just now and is retrying." };
  }
}
