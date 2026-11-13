// The app's hash routes: `#/<owner>/<repo>/<view>`, where a view is `graph`, `prs`, a
// review under `prs` (a spec branch's, `prs/<branch>` with the branch name as one encoded
// segment), followed by `/graph` or `/full/<path>`
// for its Graph tab or a file in full, `build`, `backlog`, `capabilities`, with a
// capability's page under it (`capabilities/<slug>`), or `backlog`, with a backlog item's
// page under it (`backlog/<slug>`), or `architecture`, with an
// architectural constraint's page under it (`architecture/<slug>`).

export type View = 'graph' | 'prs' | 'build' | 'backlog' | 'capabilities' | 'architecture';

const VIEWS: View[] = ['graph', 'prs', 'build', 'backlog', 'capabilities', 'architecture'];

/** What a review page reviews: a spec branch (`web/spec-branch-review`). */
export type Target = { branch: string };

export interface Route {
  owner?: string;
  repo?: string;
  view: View;
  review?: Target;
  prTab?: 'document' | 'graph';
  full?: string;
  /** The capability whose page is shown (web/capability-page). */
  capability?: string;
  /** The architectural constraint whose page is shown (web/architecture-page). */
  entry?: string;
  /** The backlog item whose page is shown. */
  item?: string;
}

function decode(part: string): string {
  try {
    return decodeURIComponent(part);
  } catch {
    return part;
  }
}

export function parseRoute(hash: string): Route {
  // Split before decoding, so a branch name's encoded slashes stay in its one segment.
  const parts = hash.replace(/^#\/?/, '').split('/').filter(Boolean).map(decode);
  const [owner, repo, view, target, ...more] = parts;
  const route: Route = { owner, repo, view: VIEWS.includes(view as View) ? (view as View) : 'graph' };
  if (route.view === 'capabilities' && target) route.capability = target;
  if (route.view === 'architecture' && target) route.entry = target;
  if (route.view === 'backlog' && target) route.item = target;
  if (route.view !== 'prs' || !target) return route;
  if (!target.startsWith('spec/')) return route;
  route.review = { branch: target };
  const [sub, ...path] = more;
  route.prTab = sub === 'graph' ? 'graph' : 'document';
  if (sub === 'full' && path.length) route.full = path.join('/');
  return route;
}

const segments = (p: string | number) => String(p).split('/').map(encodeURIComponent).join('/');

export function href(owner: string, repo: string, ...rest: (string | number)[]): string {
  return '#/' + [owner, repo, ...rest].map(segments).join('/');
}

/** The in-app link to a review, then `rest` (`graph`, or `full` and a path). */
export function reviewHref(owner: string, repo: string, t: Target, ...rest: string[]): string {
  return [href(owner, repo, 'prs'), encodeURIComponent(t.branch), ...rest.map(segments)].join('/');
}

/** The Worker's path for a review's target under the repo's API, slashes kept:
 * `branches/<branch>`. */
export function reviewPath(t: Target): string {
  return `branches/${segments(t.branch)}`;
}
