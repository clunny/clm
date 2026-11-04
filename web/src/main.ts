// The app shell: a sidebar at 900 px and wider, a bottom tab bar below, a hash router over
// the views, the offline banner (web/mobile-layout, web/installable), the data the app holds
// for the repo on screen, the approval of a spec branch (web/approve-by-branch), a decision
// on a capability or an architectural constraint (web/decide-buttons,
// web/architecture-decide-buttons), and the refresh that keeps the view current while
// the page is visible (web/live-refresh).

import './styles.css';
import { Fetched, HttpError, SignedOut, apiBase, get, post, send } from './api';
import { Proposing, renderBacklog, waiting } from './backlog';
import { renderArchitecture, renderConstraint } from './architecture';
import { Layer, ListOptions, Note, PageOptions, decide, renderCapabilities, renderCapability } from './capabilities';
import { chip, div, el } from './dom';
import { PREPARING_MS, STALE_MS, classify, failureText, retryDelay } from './errors';
import { fitGraphs } from './fit';
import { FullDocument, FullState, renderFullDocument } from './full-document';
import { Viewport, layoutGraph, queueStates, renderGraph, ringGraph } from './graph';
import { Switch, renderImplementation } from './implementation';
import { renderInspector } from './inspector';
import { LiveSocket } from './live/live';
import { Notify } from './live/notify';
import { renderPrList, renderReview } from './pr';
import { Route, Target, View, href, parseRoute, reviewHref, reviewPath } from './route';
import { ago } from './time';
import type { Architecture, Capabilities, DecisionKind, Implementation, Rating, Repo, Review, Snapshot, SpecBranch } from './types';

const INSTALL = 'https://github.com/apps/clm-spec-review/installations/new';
/** How often the view on screen is asked for again, matching the Worker's live-state cache. */
const REFRESH_MS = 60_000;

const app = document.getElementById('app')!;

/** What the app holds for the repo on screen, so a navigation within it draws at once
 * while it asks again, and a refresh has something to compare with. */
interface Held {
  base: string;
  snap: Snapshot | null;
  branches: SpecBranch[] | null;
  impl: Implementation | null;
  /** The capabilities report, once the Capabilities view asked for it. */
  caps: Capabilities | null;
  /** The architecture report, once the Architecture view asked for it. */
  arch: Architecture | null;
  /** The reviews loaded, by the Worker path they came from: the review page's, and the
   * spec branches' once the inspector asked. */
  reviews: Map<string, Review>;
  /** Of each path held, when it was fetched if it came from the offline copy. */
  from: Map<string, number | null>;
}

const empty = (base: string): Held => ({ base, snap: null, branches: null, impl: null, caps: null, arch: null, reviews: new Map(), from: new Map() });
let repos: Repo[] | null = null;
let held: Held = empty('');

/** The full-document view on screen, which listens for keys until it's left. */
let fullView: FullDocument | null = null;
/** What the last full-document view was of, and where it was, for a redraw of the same
 * document to pick up (web/live-refresh). */
let lastFull: { key: string; state: FullState } | null = null;

/** Closes the full-document view, keeping where it was for a redraw of the same document. */
function leaveFull() {
  if (fullView) lastFull = { key: lastFull?.key ?? '', state: fullView.state() };
  fullView?.close();
  fullView = null;
}
/** The backlog's node filter, for the repo it was chosen in. */
let backlogFilter: { repo: string; node: string | null } = { repo: '', node: null };
let selectedNode: string | null = null;
let buildSwitch: Switch = 'flight';
/** The Capabilities and Architecture views' "⚑ only" filters. */
const flaggedOnly: Record<Layer, boolean> = { capabilities: false, architecture: false };
/** The value chosen on a capability's or an architectural constraint's page, and what its
 * last decision said, each for the page (repo, view and slug) it was chosen on; the note only
 * while the entry is at the version it was drawn at after that decision. */
let capValue: { key: string; value: Rating } | null = null;
let capNote: { key: string; note: Note } | null = null;
/** The page (repo and slug) whose decision is in flight. */
let deciding: string | null = null;
/** Each graph's pan and zoom, by repo and page, kept across redraws. */
const viewports = new Map<string, Viewport>();
/** The writes in progress (approvals, comments, requests for changes,
 * proposals, withdrawals and decisions), during which no refresh runs, and how many have
 * ended, so a refresh begun before one ended doesn't draw what it read before it. */
let writes = 0;
let wrote = 0;
/** The reviews whose approval the Worker accepted from this page, by review path and
 * fingerprint: their button reads Approved and stays disabled. */
const accepted = new Set<string>();
/** What the last approval said, for the review it was about, kept across redraws. */
let note: { key: string; text: string } | null = null;

/** The service worker: it precaches the shell (web/installable) and shows pushes
 * (web/push-shown). */
const worker = 'serviceWorker' in navigator ? navigator.serviceWorker.register(`${import.meta.env.BASE_URL}sw.js`, { scope: import.meta.env.BASE_URL }) : Promise.reject(new Error('no service worker'));
worker.catch(() => undefined);
// The app's old worker was registered at the site's root: no browser that installed the app
// before the move keeps serving the old app over the site (web/retire-root-service-worker).
if ('serviceWorker' in navigator) {
  void navigator.serviceWorker
    .getRegistrations()
    .then((all) => Promise.all(all.filter((r) => new URL(r.scope).pathname === '/').map((r) => r.unregister())))
    .catch(() => undefined);
}

/** The Notify me switches (web/notify-switch). */
const notify = new Notify({
  // `ready`, once the registration succeeded: subscribing needs an active worker.
  pushManager: async () => ('Notification' in window && 'PushManager' in window && (await worker.then(() => true, () => false)) ? (await navigator.serviceWorker.ready).pushManager : null),
  key: async () => {
    try {
      return (await get<{ key?: string }>('/api/push/key')).data.key || null;
    } catch (e) {
      if (e instanceof HttpError && e.status === 404) return null;
      throw e;
    }
  },
  permission: () => Notification.permission,
  requestPermission: () => Notification.requestPermission(),
  subscribed: async (base, endpoint, fresh) => (await get<{ subscribed?: boolean }>(`${base}/push?endpoint=${encodeURIComponent(endpoint)}`, { live: true, fresh, keep: false })).data.subscribed === true,
  send,
  apiBase,
  online: () => !offline(),
  storage: {
    getItem: (k) => {
      try {
        return localStorage.getItem(k);
      } catch {
        return null;
      }
    },
    setItem: (k, v) => localStorage.setItem(k, v),
  },
});

function offline(): boolean {
  return [...held.from.values()].some((t) => t !== null) || !navigator.onLine;
}

/** The oldest kept response shown, when any came from the offline copy. */
function shownFrom(): number | null {
  const times = [...held.from.values()].filter((t): t is number => t !== null);
  return times.length ? Math.min(...times) : null;
}

/** Loads `path` for the view, remembering whether it came from the offline copy. */
async function load<T>(path: string): Promise<T> {
  const res: Fetched<T> = await get<T>(path);
  held.from.set(path, res.cachedAt);
  return res.data;
}

/** The Worker path of the review of `t`. */
const reviewUrl = (base: string, t: Target) => `${base}/${reviewPath(t)}/review`;

function signIn(): HTMLElement {
  return div(
    'card signin',
    div('logo', el('i', {}, '◆'), 'CLM'),
    el('h1', {}, 'Spec review'),
    el('p', {}, 'Sign in with GitHub to see the CLM repos you can read.'),
    el('a', { class: 'btn pri', href: '/auth/login' }, 'Sign in with GitHub'),
    el('p', {}, el('a', { href: INSTALL }, 'Install the GitHub App'), ' on the repos you want here.'),
  );
}

function repoPicker(list: Repo[], route: Route): HTMLSelectElement {
  const select = el('select', { class: 'repo', 'aria-label': 'Repository' });
  for (const r of list) {
    const current = r.owner === route.owner && r.name === route.repo;
    select.append(el('option', { value: `${r.owner}/${r.name}`, selected: current }, `${r.owner}/${r.name}`));
  }
  select.addEventListener('change', () => {
    const [o, r] = select.value.split('/');
    location.hash = href(o, r, route.view);
  });
  return select;
}

function shell(list: Repo[], route: Route, snap: Snapshot | null, branches: SpecBranch[] | null, main: HTMLElement): HTMLElement {
  const { owner = '', repo = '' } = route;
  const waits = snap ? waiting(snap).length : 0;
  const specPrs = (branches ?? []).length;
  const spec = snap?.spec.nodes ?? [];
  const current = spec.reduce((n, s) => n + s.current, 0);
  const total = spec.reduce((n, s) => n + s.total, 0);
  const nav: [View, string, string, number, string][] = [
    ['graph', 'Spec graph', '◇', 0, ''],
    ['prs', 'Spec PRs', '⇄', specPrs, ''],
    ['build', 'Implementation', '▶', 0, ''],
    ['backlog', 'Backlog', '☰', waits, 'wait'],
    ['capabilities', 'Capabilities', '⚖', 0, ''],
    ['architecture', 'Architecture', '▦', 0, ''],
  ];
  const tabLabel: Record<View, string> = { graph: 'Graph', prs: 'PRs', build: 'Build', backlog: 'Backlog', capabilities: 'Capabilities', architecture: 'Arch.' };
  // Without a repo, the repo views lead to the first repo.
  const to = (v: View) => (owner && repo ? href(owner, repo, v) : '#/');
  // Notify me is for the repo on screen.
  const notifySwitch = () => (owner && repo ? notify.switchFor(owner, repo) : '');

  const sidebar = el(
    'aside',
    { class: 'sidebar' },
    div('logo', el('i', {}, '◆'), 'CLM'),
    div(
      '',
      repoPicker(list, route),
      snap
        ? el('small', { class: 'repo-meta' }, `${snap.nodes.length} components · ${snap.coverage.state === 'complete' ? `${current}/${total} current` : 'coverage indexing'}`)
        : '',
      notifySwitch(),
    ),
    div(
      '',
      div('navh', 'Repository'),
      ...nav.map(([v, label, , count, cls]) =>
        el(
          'a',
          { class: v === route.view ? 'nv on' : 'nv', href: to(v), 'data-view': v },
          label,
          count ? el('span', { class: `c ${cls}`, title: v === 'backlog' ? 'Waiting on you' : '' }, String(count)) : '',
        ),
      ),
    ),
    div('', el('a', { class: 'nv', href: INSTALL }, 'Add repos')),
  );
  const tabbar = el(
    'nav',
    { class: 'tabbar', 'aria-label': 'Views' },
    ...nav.map(([v, , icon, count]) =>
      el(
        'a',
        { class: v === route.view ? 'on' : '', href: to(v), 'data-view': v },
        el('b', {}, icon),
        tabLabel[v],
        count && (v === 'backlog' || v === 'prs') ? el('span', { class: 'bdg' }, String(count)) : '',
      ),
    ),
  );
  const mtop = div('mtop', repoPicker(list, route), notifySwitch());
  const now = Math.floor(Date.now() / 1000);
  const from = shownFrom();
  const stale = refreshFailing && Date.now() - lastGood >= STALE_MS;
  const banner =
    from !== null
      ? div('offline', `${navigator.onLine ? "CLM couldn't be reached" : 'Offline'} · showing data from ${ago(Math.floor(from / 1000), now)}`)
      : !navigator.onLine
        ? div('offline', 'Offline')
        : stale
          ? div('offline stale', `Couldn't refresh · showing data from ${ago(Math.floor(lastGood / 1000), now)}`)
          : '';
  return div('app', sidebar, el('main', { class: 'main' }, mtop, banner, main), tabbar);
}

/** The reviews of the spec branches, kept in `held`, so only the missing ones are loaded;
 * any that fails to load is left out. */
async function specReviews(base: string, branches: SpecBranch[]): Promise<Review[]> {
  const urls = branches.map((b) => reviewUrl(base, { branch: b.branch }));
  await Promise.all(
    urls
      .filter((u) => !held.reviews.has(u))
      .map((u) =>
        load<Review>(u)
          .then((r) => held.reviews.set(u, r))
          .catch(() => undefined),
      ),
  );
  return urls.map((u) => held.reviews.get(u)).filter((r): r is Review => r !== undefined);
}

/** `make(impl)`, drawn at once from the implementation state held, or without it, and
 * drawn again when it arrives. */
function withImpl(base: string, make: (impl: Implementation | null) => HTMLElement): HTMLElement {
  const wrap = div('', make(held.impl));
  if (!held.impl) {
    const b = held;
    load<Implementation>(`${base}/implementation`)
      .then((impl) => {
        if (held !== b) return;
        held.impl = impl;
        wrap.replaceChildren(make(impl));
      })
      // Without it the view stays as drawn; the next refresh asks again.
      .catch(() => undefined);
  }
  return wrap;
}

/** The graph and its inspector. The graph doesn't wait for the implementation state: when
 * it arrives, the cards take their queue-state rings and the inspector shows the work in
 * flight. */
async function graphPage(base: string, snap: Snapshot, touched: Set<string>, key: string, openPrs: () => Promise<Review[]>): Promise<HTMLElement> {
  const layout = await layoutGraph(snap.spec);
  let prs: Review[] = [];
  let loading: Promise<void> | null = null;
  const inspector = () =>
    selectedNode
      ? renderInspector(snap, held.impl, prs, selectedNode, () => select(null))
      : el('aside', { class: 'insp card closed' }, div('sec', div('note', 'Select a component to see its contract, changes, work and backlog.')));
  // Selecting a node swaps only the inspector, so the graph keeps its pan and zoom.
  const graph = renderGraph(snap, touched, layout, (id) => select(id), selectedNode ?? undefined, viewports.get(key), (v) => viewports.set(key, v), queueStates(held.impl));
  let insp = inspector();
  function redraw() {
    const next = inspector();
    insp.replaceWith(next);
    insp = next;
  }
  function select(id: string | null) {
    selectedNode = id;
    graph.querySelectorAll<HTMLElement>('.n, .grp').forEach((n) => n.classList.toggle('sel', n.dataset.node === id));
    redraw();
    loading ??= openPrs()
      .then((r) => {
        prs = r;
        if (selectedNode) redraw();
      })
      .catch(() => undefined);
  }
  if (selectedNode) select(selectedNode);
  if (!held.impl) {
    const b = held;
    load<Implementation>(`${base}/implementation`)
      .then((impl) => {
        if (held !== b) return;
        held.impl = impl;
        ringGraph(graph, queueStates(impl));
        if (selectedNode) redraw();
      })
      .catch(() => undefined);
  }
  return div('gwrap', graph, insp);
}

interface PageContext {
  /** A full-document view this page opened, for `show` to close when it's left. */
  full: FullDocument | null;
}

/** A spec branch's review, with Approve (web/spec-branch-review, web/approve-by-branch). */
async function prPage(route: Route, base: string, snap: Snapshot, first: Review, ctx: PageContext): Promise<HTMLElement> {
  const { owner = '', repo = '' } = route;
  const t = route.review!;
  const url = reviewUrl(base, t);
  let review = first;
  const name = t.branch;
  if (route.full) {
    const doc = review.documents.find((d) => d.path === route.full);
    if (!doc) return div('empty', `${route.full} isn't changed in ${name}.`);
    const back = el('a', { href: reviewHref(owner, repo, t) }, `‹ ${name}`);
    // Only a redraw of this same document picks up where the last view was.
    const key = `${url}/full/${route.full}`;
    const prev = lastFull?.key === key ? lastFull.state : null;
    lastFull = { key, state: prev ?? { mode: 'inline', index: 0, text: '' } };
    const view = renderFullDocument(doc, prev?.mode ?? 'inline', back);
    ctx.full = view;
    requestAnimationFrame(() => (prev ? view.restore(prev) : view.go(0)));
    return view.el;
  }
  const page = div('');
  // The review as it was shown: its path and fingerprint.
  const shownKey = () => `${url}#${review.fingerprint}`;
  // A note is kept only for the review it was about, so a later visit to a moved branch
  // doesn't show an old outcome.
  const status = div('note', note?.key === shownKey() ? note.text : '');
  const say = (text: string) => {
    note = { key: shownKey(), text };
    status.textContent = text;
  };
  /** The review again, from the Worker and never the copy kept for offline use, so what is
   * shown is the ledger's and the next write pins what is then on screen; false when it
   * couldn't be loaded. */
  const reload = async () => {
    try {
      review = (await get<Review>(url, { live: true, fresh: crypto.randomUUID() })).data;
      held.reviews.set(url, review);
      held.from.set(url, null);
      return true;
    } catch {
      return false;
    }
  };
  const MOVED = 'The branch changed since this review was shown: here it is as it is now. Review it again before approving.';
  /** Posts a comment or a request for changes (web/comment-on-review, web/request-changes):
   * null once the Worker took it, else why not, for the form to show. On success, and when
   * the branch moved (409), the review is loaded again and drawn once the form has settled,
   * so a sent form closes and a refused one keeps its text for the review now on screen. */
  const write = (path: 'comments' | 'request-changes', body: object): Promise<string | null> =>
    whileWriting(async () => {
      try {
        const res = await post(`${base}/${reviewPath(t)}/${path}`, body);
        if (res.ok || res.status === 409) {
          const moved = res.status === 409;
          if (moved) say(path === 'comments' ? 'The branch changed since this review was shown: here it is as it is now.' : MOVED);
          if (!(await reload())) say(moved ? 'The branch changed since this review was shown, and its new review could not be loaded. Reload.' : 'Sent. Reload to see the review as it is now.');
          // After the form has closed, or kept its draft: a timer runs after every pending
          // promise callback.
          setTimeout(() => void render());
          return moved ? 'Not sent: the branch changed since this review was shown. Read it again, then send.' : null;
        }
        if (res.status === 403) return `Not sent: ${path === 'comments' ? 'commenting' : 'requesting changes'} needs ${path === 'comments' ? 'read' : 'write'} access to this repository.`;
        if (res.status === 422) return `Not sent: the Worker refused the ${await refusedField(res)}.`;
        return `Not sent (${res.status}).`;
      } catch (e) {
        if (e instanceof SignedOut) showSignIn();
        return notSent(e);
      }
    });
  const act = async () => {
    say('Approving…');
    await whileWriting(async () => {
      try {
        const res = await post(`${base}/${reviewPath(t)}/approve`, { fingerprint: review.fingerprint });
        // The repository became signed after the review was drawn (web/signed-approvals).
        const signed = res.status === 422 && (await res.json().catch(() => null))?.error === 'signed';
        if (res.ok) {
          accepted.add(shownKey());
          say('Approved.');
        } else if (res.status === 409) {
          say(MOVED);
        } else if (signed) {
          say('Not approved: this repository counts only approvals signed with your SSH key.');
        } else {
          say(`Not approved (${res.status}).`);
        }
        // The review again, from the Worker and never the copy kept for offline use, so the
        // approval shown is the ledger's and the next Approve pins what is then on screen.
        if (res.ok || res.status === 409 || signed) {
          const text = status.textContent ?? '';
          if (await reload()) say(text);
          else {
            // The outcome stands; only the review on screen couldn't be brought up to date.
            say(
              res.status === 409
                ? 'The branch changed since this review was shown, and its new review could not be loaded. Reload before approving.'
                : `${text} Reload to see the review as it is now.`,
            );
          }
        }
      } catch (e) {
        say(e instanceof SignedOut ? `Signed out. Sign in again to approve.` : `Offline: approve when back online.`);
      }
      // Drawn again, so the button reflects the outcome: disabled once accepted, else enabled.
      await render();
    });
  };
  const tabs = div(
    'tabs',
    el('a', { class: route.prTab === 'document' ? 'tab on' : 'tab', href: reviewHref(owner, repo, t) }, 'Document'),
    el('a', { class: route.prTab === 'graph' ? 'tab on' : 'tab', href: reviewHref(owner, repo, t, 'graph') }, 'Graph'),
  );
  const render = async () => {
    const view = renderReview(review, {
      offline: offline(),
      accepted: accepted.has(shownKey()),
      onAct: () => void act(),
      fullHref: (p) => reviewHref(owner, repo, t, 'full', p),
      scope: url,
      onComment: (target, text) => write('comments', { fingerprint: review.fingerprint, target, text }),
      onRequest: (text) => write('request-changes', { fingerprint: review.fingerprint, text }),
    });
    if (route.prTab === 'graph') {
      const head = view.querySelector('.head')!;
      const prhead = view.querySelector('.prhead')!;
      const graph = await graphPage(base, snap, new Set(review.nodes.map((x) => x.id)), `${base}|pr:${url}`, async () => [review]);
      page.replaceChildren(view.querySelector('.crumb')!, head, tabs, prhead.querySelector('.approve-bar')!, status, graph);
    } else {
      const doc = view.querySelector('.doc')!;
      view.insertBefore(tabs, doc);
      view.insertBefore(status, doc);
      page.replaceChildren(view);
    }
  };
  await render();
  return page;
}

/** Runs a write: no refresh runs while it does, and a notice that came meanwhile refreshes
 * once it ends, however it ends. */
async function whileWriting<T>(go: () => Promise<T>): Promise<T> {
  writes++;
  try {
    return await go();
  } finally {
    writes--;
    wrote++;
    pumpNotice();
  }
}

/** The field a 422's `{"error"}` names, or `request` when it names none. */
async function refusedField(res: Response): Promise<string> {
  const field = (await res.json().catch(() => null))?.error;
  return typeof field === 'string' ? field : 'request';
}

/** Why a write that threw wasn't sent. */
function notSent(e: unknown): string {
  return e instanceof SignedOut ? 'Signed out. Sign in again to send it.' : 'Not sent: offline.';
}

/** Asks the Worker for the implementation state again, never the copy kept for offline use,
 * and draws the view on screen anew; a failure leaves the state for the next refresh. */
async function reloadImpl(base: string) {
  try {
    const impl = (await get<Implementation>(`${base}/implementation`, { live: true, fresh: crypto.randomUUID() })).data;
    if (held.base !== base) return;
    held.impl = impl;
    held.from.set(`${base}/implementation`, null);
  } catch {
    // The next refresh asks again.
  }
  if (held.base === base) await draw(parseRoute(location.hash), showing);
}

/** Proposes an item for the backlog (web/propose-from-backlog): null once the Worker took it
 * and the implementation state was asked for again, else why not, for the form to show. */
function propose(base: string, title: string, body: string, nodes: string[]): Promise<string | null> {
  return whileWriting(async () => {
    try {
      const res = await post(`${base}/proposals`, { title, body, nodes });
      if (res.ok) {
        await reloadImpl(base);
        return null;
      }
      if (res.status === 409) return 'That title is taken: choose another.';
      if (res.status === 422) return `Not proposed: the Worker refused the ${await refusedField(res)}.`;
      return `Not proposed (${res.status}).`;
    } catch (e) {
      if (e instanceof SignedOut) showSignIn();
      return notSent(e);
    }
  });
}

/** Withdraws proposal `id`: null once the Worker took it and the implementation state was
 * asked for again, else why not, shown under the proposal. */
function withdraw(base: string, id: string): Promise<string | null> {
  return whileWriting(async () => {
    try {
      const res = await send('DELETE', `${base}/proposals/${encodeURIComponent(id)}`);
      if (!res.ok) return `Not withdrawn (${res.status}).`;
    } catch (e) {
      if (e instanceof SignedOut) showSignIn();
      return e instanceof SignedOut ? 'Signed out.' : 'Not withdrawn: offline.';
    }
    await reloadImpl(base);
    return null;
  });
}

/** The backlog, drawn from the snapshot first and marked with what holds each item once
 * the implementation state arrives (web/backlog-taken), with New proposal and Proposed
 * (web/propose-from-backlog). */
function backlogPage(base: string, snap: Snapshot): HTMLElement {
  if (backlogFilter.repo !== snap.repo) backlogFilter = { repo: snap.repo, node: null };
  return withImpl(base, (impl) => {
    const wrap = div('');
    const proposing: Proposing = {
      proposals: impl ? (impl.proposals ?? []) : null,
      offline: offline(),
      onPropose: (title, body, nodes) => propose(base, title, body, nodes),
      onWithdraw: (id) => withdraw(base, id),
    };
    // Filtering redraws only the backlog, from what is already loaded.
    const draw = () =>
      wrap.replaceChildren(
        renderBacklog(
          snap,
          impl?.taken ?? null,
          backlogFilter.node,
          (node) => {
            backlogFilter = { repo: snap.repo, node };
            draw();
          },
          proposing,
        ),
      );
    draw();
    return wrap;
  });
}

/** The report of `layer` again, from the Worker and never the copy kept for offline use, so
 * the version on screen is the one the next decision names, read with `fresh` so the Worker
 * reads the ledgers anew rather than its minute-old copy; false when it couldn't. */
async function reloadReport(base: string, layer: Layer): Promise<boolean> {
  try {
    const path = `${base}/${layer}`;
    const report = (await get<Capabilities | Architecture>(path, { live: true, fresh: crypto.randomUUID() })).data;
    if (held.base !== base) return false;
    if (layer === 'capabilities') held.caps = report as Capabilities;
    else held.arch = report as Architecture;
    held.from.set(path, null);
    return true;
  } catch {
    return false;
  }
}

/** The Capabilities or Architecture list (web/capabilities-list, web/architecture-list), or
 * an entry's page with its decision (web/capability-page, web/decide-buttons,
 * web/architecture-page, web/architecture-decide-buttons), drawn from the report `held` has,
 * which `page` checked: what is held for this repo, kept so that a redraw after another repo
 * was opened still draws this one's. */
function reviewPage(layer: Layer, route: Route, base: string): HTMLElement {
  const { owner = '', repo = '' } = route;
  const h = held;
  const wrap = div('');
  const slug = layer === 'capabilities' ? route.capability : route.entry;
  const list = (o: ListOptions) => (layer === 'capabilities' ? renderCapabilities(h.caps!, o) : renderArchitecture(h.arch!, o));
  if (slug === undefined) {
    const drawList = () =>
      wrap.replaceChildren(
        list({
          flaggedOnly: flaggedOnly[layer],
          onFilter: (on) => {
            flaggedOnly[layer] = on;
            drawList();
          },
          link: (s) => href(owner, repo, layer, s),
        }),
      );
    drawList();
    return wrap;
  }
  const key = `${base}|${layer}|${slug}`;
  const entries = () => (layer === 'capabilities' ? h.caps?.report.capabilities : h.arch?.architecture) ?? [];
  const version = () => entries().find((c) => c.slug === slug)?.version ?? '';
  const one = (o: PageOptions) => (layer === 'capabilities' ? renderCapability(h.caps!, slug, o) : renderConstraint(h.arch!, slug, o));
  const render = () =>
    wrap.replaceChildren(
      one({
        offline: offline(),
        busy: deciding === key,
        value: capValue?.key === key ? capValue.value : null,
        onValue: (value) => {
          capValue = { key, value };
          render();
        },
        onDecide: (d) => void act(d),
        back: href(owner, repo, layer),
        note: capNote?.key === `${key}@${version()}` ? capNote.note : null,
      }),
    );
  const act = (d: DecisionKind) =>
    whileWriting(async () => {
      const value = capValue?.key === key ? capValue.value : null;
      if (value === null || deciding !== null) return;
      deciding = key;
      try {
        const note = await decide(base, layer, slug, version(), value, d, { post, reload: () => reloadReport(base, layer), signedOut: showSignIn });
        capNote = { key: `${key}@${version()}`, note };
      } finally {
        deciding = null;
      }
      if (held.base === base && !signedOutShown) render();
    });
  render();
  return wrap;
}

function buildPage(impl: Implementation, snap: Snapshot): HTMLElement {
  const wrap = div('');
  const draw = () =>
    wrap.replaceChildren(
      renderImplementation(impl, snap, {
        now: Math.floor(Date.now() / 1000),
        show: buildSwitch,
        onShow: (which, node) => {
          buildSwitch = which;
          draw();
          if (node) wrap.querySelector(`.fn[data-node="${CSS.escape(node)}"]`)?.scrollIntoView({ block: 'center' });
        },
      }),
    );
  draw();
  return wrap;
}

/** The repo's page for `route`, from what `held` has: the snapshot and spec branches, and
 * for Build the implementation state and for a review page its review, which `show` loads
 * first. */
async function page(route: Route, snap: Snapshot, branches: SpecBranch[], ctx: PageContext): Promise<HTMLElement> {
  const base = held.base;
  switch (route.view) {
    case 'graph': {
      const g = await graphPage(base, snap, new Set(), `${base}|graph`, () => specReviews(base, branches));
      return div(
        '',
        div('crumb', snap.repo),
        div('head', div('', el('h1', {}, 'Spec graph'), div('meta', `at ${snap.sha.slice(0, 8)}`, snap.violations.length ? chip('hard', `${snap.violations.length} violations`) : ''))),
        g,
      );
    }
    case 'prs': {
      if (route.review === undefined) {
        const link = (t: Target) => reviewHref(route.owner!, route.repo!, t);
        return renderPrList(branches, link);
      }
      const review = held.reviews.get(reviewUrl(base, route.review));
      return review ? prPage(route, base, snap, review, ctx) : errorView(new Error('no review'));
    }
    case 'build':
      return held.impl ? buildPage(held.impl, snap) : errorView(new Error('no implementation state'));
    case 'backlog':
      return backlogPage(base, snap);
    case 'capabilities':
      return held.caps ? reviewPage('capabilities', route, base) : errorView(new Error('no capabilities report'));
    case 'architecture':
      return held.arch ? reviewPage('architecture', route, base) : errorView(new Error('no architecture report'));
  }
}

/** The paths the view on screen is drawn from, beyond the snapshot and spec branches: the
 * implementation state for the graph, Build, Backlog and a review's Graph tab, and a review
 * page's review. */
function needs(route: Route, base: string): { impl: boolean; review: string | null; caps: boolean; arch: boolean } {
  const review = route.view === 'prs' && route.review ? reviewUrl(base, route.review) : null;
  return {
    impl: route.view === 'build' || route.view === 'graph' || route.view === 'backlog' || (review !== null && route.prTab === 'graph'),
    review,
    caps: route.view === 'capabilities',
    arch: route.view === 'architecture',
  };
}

/** Fetches what `route` needs that `held` doesn't have yet. */
async function fill(route: Route) {
  // Filled into what is held for this repo, even when another repo was opened meanwhile.
  const h = held;
  const base = h.base;
  const need = needs(route, base);
  const [snap, branches, review, caps, arch] = await Promise.all([
    h.snap ?? load<Snapshot>(`${base}/snapshot`),
    h.branches ?? load<SpecBranch[]>(`${base}/branches`).catch(() => [] as SpecBranch[]),
    need.review !== null && !h.reviews.has(need.review) ? load<Review>(need.review) : null,
    need.caps && !h.caps ? load<Capabilities>(`${base}/capabilities`) : null,
    need.arch && !h.arch ? load<Architecture>(`${base}/architecture`) : null,
  ]);
  if (caps) h.caps = caps;
  if (arch) h.arch = arch;
  h.snap = snap;
  h.branches = branches;
  if (review && need.review !== null) h.reviews.set(need.review, review);
  // Build can't draw without the implementation state; the graph draws before it arrives.
  if (route.view === 'build' && !h.impl) h.impl = await load<Implementation>(`${base}/implementation`);
}

let showing = 0;
/** The ask-again after a failure (web/honest-errors), and how many have failed in a row. */
let retryTimer: ReturnType<typeof setTimeout> | null = null;
let attempts = 0;
/** When the view on screen was last known current, and whether its refreshes are failing. */
let lastGood = Date.now();
let refreshFailing = false;

/** The view is drawn from the kept copy: ask for live data with backoff until it comes. */
function retryKept(ticket: number) {
  retryTimer = setTimeout(async () => {
    await startRefresh();
    if (ticket === showing && shownFrom() !== null && navigator.onLine) retryKept(ticket);
  }, retryDelay(attempts++));
}
/** Fits the graph on screen to the window, now; it is also kept fitted as the page changes
 * (web/graph-fills-height). */
const fitGraph = fitGraphs(app);

/** Draws `route` from `held`, in place of whatever is on screen, keeping the scroll. */
async function draw(route: Route, ticket: number) {
  const ctx: PageContext = { full: null };
  leaveFull();
  const main = await page(route, held.snap!, held.branches ?? [], ctx).catch((e) => errorView(e));
  if (ticket !== showing) {
    // Left before it was drawn: whatever it opened is never shown.
    ctx.full?.close();
    return;
  }
  fullView = ctx.full;
  const y = scrollY;
  // A form being typed in keeps its focus and caret across the redraw.
  const active = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  const typing = active?.dataset?.draft ? { key: active.dataset.draft, at: active.selectionStart, to: active.selectionEnd } : null;
  app.replaceChildren(shell(repos ?? [], route, held.snap, held.branches, main));
  // Fitted before the scroll is restored, so a graph never shows at its fallback height.
  fitGraph();
  if (typing) {
    const again = app.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[data-draft="${CSS.escape(typing.key)}"]`);
    again?.focus({ preventScroll: true });
    if (again && typing.at !== null) again.setSelectionRange(typing.at, typing.to);
  }
  scrollTo(0, y);
  signedOutShown = false;
  syncLive();
}

/** The loading bar, in the page from its first paint: shown from a navigation's start
 * until its view is drawn, never for a refresh (web/loading-indicator). */
const loadbar = document.getElementById('loadbar');
const loading = (on: boolean) => loadbar?.toggleAttribute('hidden', !on);

/** A navigation, with the loading bar, or with `nav` false a redraw for going online or
 * offline, without it: draws the view from what is held, asking for the rest. */
async function show(nav = true) {
  const ticket = ++showing;
  const route = parseRoute(location.hash);
  if (nav) loading(true);
  if (retryTimer !== null) clearTimeout(retryTimer);
  retryTimer = null;
  if (nav) {
    attempts = 0;
    refreshFailing = false;
  }
  // Sent on to the first repo, whose navigation ends the bar.
  let redirected = false;
  try {
    // The repo list is asked for on navigation only: a held one draws the shell at once
    // while it is asked again.
    const hadRepos = repos !== null;
    const list = repos ?? (await load<Repo[]>('/api/repos'));
    repos = list;
    // A later navigation owns the screen and the route: this one does nothing more.
    if (ticket !== showing) return;
    if (!route.owner || !route.repo) {
      leaveFull();
      if (list.length === 0) {
        app.replaceChildren(
          div(
            'card signin',
            el('h1', {}, 'No CLM repos yet'),
            el('p', {}, 'Install the GitHub App on a repo that has specs/SPEC.md.'),
            el('a', { class: 'btn pri', href: INSTALL }, 'Install the GitHub App'),
          ),
        );
        return;
      }
      redirected = true;
      location.replace(href(list[0].owner, list[0].name, 'graph'));
      return;
    }
    const base = apiBase(route.owner, route.repo);
    // A node selected in one repo means nothing in another, and nothing held there helps.
    if (base !== held.base) {
      held = empty(base);
      selectedNode = null;
    }
    const drawn = held.snap !== null;
    await fill(route);
    if (ticket !== showing) return;
    await draw(route, ticket);
    // Drawn from what was held: ask again now, as a refresh does.
    if (hadRepos) void refreshRepos(route, ticket);
    if (drawn) startRefresh();
    if (shownFrom() === null) {
      attempts = 0;
      lastGood = Date.now();
    } else if (navigator.onLine) {
      // Drawn from the copy kept for offline use while CLM can't be reached: ask again.
      retryKept(ticket);
    }
  } catch (e) {
    if (ticket !== showing) return;
    leaveFull();
    if (e instanceof SignedOut) {
      showSignIn();
    } else {
      const f = classify(e);
      app.replaceChildren(div('main', errorView(e)));
      if (f.kind === 'preparing') retryTimer = setTimeout(() => void show(false), PREPARING_MS);
      else if (f.kind === 'retry') retryTimer = setTimeout(() => void show(false), retryDelay(attempts++));
    }
  } finally {
    if (ticket === showing) {
      syncLive();
      if (!redirected) loading(false);
    }
  }
}

/** Asks for the repo list again and redraws the shell when it differs; a failure leaves it. */
async function refreshRepos(route: Route, ticket: number) {
  try {
    const list = (await get<Repo[]>('/api/repos', { live: true })).data;
    if (ticket !== showing || JSON.stringify(list) === JSON.stringify(repos)) return;
    repos = list;
    await draw(route, ticket);
  } catch (e) {
    if (e instanceof SignedOut && ticket === showing) showSignIn();
  }
}

/** A JSON value with `now` left out, for comparing implementation states. */
function sans(v: unknown, key: string): unknown {
  return JSON.stringify(v, (k, x) => (k === key ? undefined : x));
}

/** Asks again for the endpoints the view on screen was drawn from, and redraws it in place
 * when an answer differs; otherwise leaves it as it is. A failure leaves it too, except that
 * being signed out shows the sign-in page. */
async function refresh(fresh?: string) {
  if (writes > 0 || document.visibilityState !== 'visible' || !held.snap) return;
  const route = parseRoute(location.hash);
  if (!route.owner || !route.repo || apiBase(route.owner, route.repo) !== held.base) return;
  const base = held.base;
  const ticket = showing;
  const since = wrote;
  const need = needs(route, base);
  const live = <T>(path: string) => get<T>(path, { live: true, fresh }).then((r) => r.data);
  void notify.refresh(fresh);
  try {
    // The review page's review, and on the graph the spec branches' reviews the inspector loaded.
    const branchReviews = route.view === 'graph' ? (held.branches ?? []).map((b) => reviewUrl(base, { branch: b.branch })) : [];
    const reviewUrls = [...held.reviews.keys()].filter((u) => u === need.review || branchReviews.includes(u));
    const [snap, branches, impl, caps, arch, ...reviews] = await Promise.all([
      live<Snapshot>(`${base}/snapshot`),
      live<SpecBranch[]>(`${base}/branches`),
      need.impl ? live<Implementation>(`${base}/implementation`) : null,
      need.caps ? live<Capabilities>(`${base}/capabilities`) : null,
      need.arch ? live<Architecture>(`${base}/architecture`) : null,
      ...reviewUrls.map((u) => live<Review>(u).then((r) => [u, r] as const)),
    ]);
    if (ticket !== showing || base !== held.base || writes > 0 || wrote !== since) return;
    let changed = JSON.stringify(snap) !== JSON.stringify(held.snap) || JSON.stringify(branches) !== JSON.stringify(held.branches);
    if (impl && sans(impl, 'now') !== sans(held.impl, 'now')) changed = true;
    if (caps && JSON.stringify(caps) !== JSON.stringify(held.caps)) changed = true;
    if (arch && JSON.stringify(arch) !== JSON.stringify(held.arch)) changed = true;
    for (const [u, r] of reviews) if (JSON.stringify(r) !== JSON.stringify(held.reviews.get(u))) changed = true;
    // Everything on screen is now live: an offline banner drawn from kept copies goes.
    const wasOffline = offline() || refreshFailing;
    held.from.clear();
    refreshFailing = false;
    lastGood = Date.now();
    if (!changed && !wasOffline) return;
    held.snap = snap;
    held.branches = branches;
    if (impl) held.impl = impl;
    if (caps) held.caps = caps;
    if (arch) held.arch = arch;
    for (const [u, r] of reviews) held.reviews.set(u, r);
    if (selectedNode && !snap.spec.nodes.some((n) => n.node === selectedNode)) selectedNode = null;
    await draw(route, ticket);
  } catch (e) {
    if (ticket !== showing) return;
    if (e instanceof SignedOut) showSignIn();
    else {
      // The view stays as it is; once it is 3 minutes old it says so.
      refreshFailing = true;
      // Redrawn on each failure from then on, so the age on screen keeps up.
      if (Date.now() - lastGood >= STALE_MS) await draw(route, ticket);
    }
  }
}

/** The refresh running, if one is: only one runs at a time. */
let refreshing: Promise<void> | null = null;
/** A live notice not yet refreshed for: its delivery, or null for one that names none.
 * Notices that arrive during a refresh share the next one. */
let noticed: string | null | undefined;

/** Starts a refresh, unless one is running. */
function startRefresh(fresh?: string): Promise<void> {
  if (refreshing) {
    // The one running may be for a view since left: run another once it ends.
    noticed ??= null;
    return refreshing;
  }
  return (refreshing = refresh(fresh).finally(() => {
    refreshing = null;
    pumpNotice();
  }));
}

/** Refreshes for the notice waiting, once no refresh or write runs. */
function pumpNotice() {
  if (refreshing || writes > 0 || noticed === undefined) return;
  const d = noticed;
  noticed = undefined;
  startRefresh(d ?? undefined);
}

/** The repo's live socket (web/live-updates): each notice refreshes the view at once. */
const live = new LiveSocket(
  (url) => new WebSocket(url),
  (delivery) => {
    noticed = delivery ?? null;
    pumpNotice();
  },
);

/** The sign-in page is on screen: the socket stays closed until a view is drawn again. */
let signedOutShown = false;

function showSignIn() {
  leaveFull();
  signedOutShown = true;
  live.want(null);
  app.replaceChildren(signIn());
}

/** Holds the socket for the repo on screen while the page is visible and online. */
function syncLive() {
  const route = parseRoute(location.hash);
  const onScreen = !signedOutShown && document.visibilityState === 'visible' && navigator.onLine && held.snap !== null && !!route.owner && !!route.repo && apiBase(route.owner, route.repo) === held.base;
  live.want(onScreen ? `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}${held.base}/live` : null);
}

function errorView(e: unknown): HTMLElement {
  if (e instanceof SignedOut) return signIn();
  const { title, text } = failureText(classify(e));
  return div('card signin', el('h1', {}, title), el('p', {}, text));
}

let timer: ReturnType<typeof setInterval> | null = null;

/** Refreshes about every 60 seconds while the page is visible, and at once when it becomes
 * visible again; nothing is asked while it is hidden. */
function schedule() {
  if (timer !== null) clearInterval(timer);
  timer = document.visibilityState === 'visible' ? setInterval(() => startRefresh(), REFRESH_MS) : null;
}

addEventListener('hashchange', () => show());
addEventListener('online', () => show(false));
addEventListener('offline', () => show(false));
document.addEventListener('visibilitychange', () => {
  schedule();
  syncLive();
  if (document.visibilityState === 'visible') startRefresh();
});
schedule();
show();
