// The implementation view (Build): counts, then a switch between In flight, the `view`
// node's `flight` model, and Blockers, its `blockers` model, as the Worker computed them,
// and the activity feed (web/implementation-view, web/blockers-view, web/activity-feed).

import { Child, chip, code, div, el } from './dom';
import { ago, lapsesIn } from './time';
import type { Activity, FlightChange, FlightNode, Implementation, Root, Snapshot } from './types';

export type Switch = 'flight' | 'blockers';

export interface BuildOptions {
  /** The browser's clock, in Unix seconds: relative times are drawn against it, so they
   * stay current between redraws (web/live-refresh). */
  now: number;
  show: Switch;
  /** Called to switch panels, and, from a Blockers link, to show a node's In flight entry. */
  onShow: (show: Switch, node?: string) => void;
}

const STATE_CHIP: Record<string, [string, string]> = {
  ready: ['acc', 'Ready'],
  claimed: ['acc', 'Claimed'],
  blocked: ['hard', 'Blocked'],
};
/** The queue states, in the order the totals come in (`builds-in-subagents`,
 * `needs-alone-order`). */
const STATE_ORDER = ['claimed', 'ready', 'blocked'];
const PENDING_ORDER = ['behind', 'unbuilt', 'obsolete'];
/** Backlog kinds that wait on a person (`requirement-blocks`). */
const HUMAN = new Set(['question', 'human-task']);

/** `<n> <state>` for each state with a count, known ones first in their order. */
function byState(counts: Record<string, number>, order: string[]): string {
  const keys = [...order.filter((k) => counts[k]), ...Object.keys(counts).filter((k) => !order.includes(k) && counts[k])];
  return keys.map((k) => `${counts[k]} ${k}`).join(', ');
}

/** A link to node `id`'s entry in In flight, labelled `label`. */
function nodeLink(id: string, o: BuildOptions, label: Child = id): HTMLElement {
  const a = el('a', { href: `#node-${id}`, class: 'nl' }, label);
  a.addEventListener('click', (e) => {
    e.preventDefault();
    o.onShow('flight', id);
  });
  return a;
}

function pending(n: FlightNode): HTMLElement {
  const counts: Record<string, number> = {};
  for (const p of n.pending) counts[p.state] = (counts[p.state] ?? 0) + 1;
  return div(
    'pend',
    el('span', { class: 'pc' }, `${n.pending.length} pending`),
    n.pending.length ? ` · ${byState(counts, PENDING_ORDER)}` : '',
    el(
      'ul',
      { class: 'reqs' },
      ...n.pending.map((p) => {
        const by = [...p.blocked_by.map((s) => `backlog/${s}`), ...p.waits_on];
        return el('li', { class: `rq ${p.state}` }, code(p.id), ` ${p.state}`, by.length ? ` · blocked by ${by.join(', ')}` : '');
      }),
    ),
  );
}

function nodeEntry(n: FlightNode, impl: Implementation, snap: Snapshot, o: BuildOptions): HTMLElement {
  const [cls, label] = STATE_CHIP[n.state] ?? ['mute', n.state];
  const lines: Child[][] = [];
  if (n.claim) {
    const lastPush = n.claim.lapses_at - impl.expiry_hours * 3600;
    lines.push(['Claimed on ', code(`impl/${n.node}`), ` · last push ${ago(lastPush, o.now)} · lapses in ${lapsesIn(n.claim.lapses_at, o.now)}`]);
  }
  // The branch's page, and whether the ledger holds a `checked` record for its head;
  // nothing about the check while the Worker doesn't yet know.
  const page = impl.pages?.[`impl/${n.node}`];
  if (page) {
    const checked = page.checked === undefined ? '' : el('span', { class: 'checked' }, page.checked ? 'checked' : 'not checked');
    lines.push(['Page ', el('a', { class: 'page', href: page.url, target: '_blank', rel: 'noopener' }, `#${page.number}`), checked ? ' · ' : '', checked]);
  }
  // A backlog item, or a requirement of another node it needs (`requirement-needs`), which
  // links to that node's entry.
  for (const b of n.blocked_by) {
    const item = snap.backlog.find((x) => `backlog/${x.slug}` === b.item);
    const name = b.requirement ? nodeLink(b.requirement.split('/')[0], o, code(b.requirement)) : code(b.item ?? '');
    lines.push(['Blocked by ', name, item && HUMAN.has(item.kind) ? chip('wait', 'waits on you') : '', ` · ${b.requirements.join(', ')}`]);
  }
  const lease = n.claim
    ? [div('lease', el('i', { style: `width:${Math.max(0, Math.min(100, ((n.claim.lapses_at - o.now) / (impl.expiry_hours * 3600)) * 100))}%` }))]
    : [];
  return el(
    'div',
    { class: `fn ${n.state}`, id: `node-${n.node}`, 'data-node': n.node },
    div('top', el('b', {}, n.node), chip('mute', n.priority), chip(cls, label)),
    lines.length ? div('d', ...lines.flatMap((l, i) => (i ? [el('br'), ...l] : l))) : '',
    pending(n),
    ...lease,
  );
}

function changeRow(c: FlightChange): HTMLElement {
  const share = c.total ? Math.round((c.current / c.total) * 100) : 0;
  return el(
    'div',
    { class: 'chg', 'data-change': c.change },
    div('top', el('b', {}, c.change), chip('mute', c.priority), c.complete ? chip('ok', 'complete') : '', c.obsolete.length ? chip('hard', `${c.obsolete.length} obsolete`) : ''),
    div('prog', div('bar', el('i', { style: `width:${share}%` })), el('span', {}, `${c.current}/${c.total} requirements current`)),
  );
}

function inFlight(impl: Implementation, snap: Snapshot, o: BuildOptions): HTMLElement {
  const f = impl.flight;
  return div(
    'flight',
    div(
      'card sec',
      el('h3', { class: 'h' }, 'Components'),
      ...(f.nodes.length ? f.nodes.map((n) => nodeEntry(n, impl, snap, o)) : [div('note', 'Nothing is pending: every component is up to its spec.')]),
    ),
    div(
      'card sec',
      el('h3', { class: 'h' }, 'Changes'),
      ...(f.changes.length
        ? f.changes.map((g) => el('div', { class: 'grp-changes', 'data-status': g.status }, div('gs', g.status), ...g.changes.map(changeRow)))
        : [div('note', 'No live changes.')]),
    ),
  );
}

function rootCard(r: Root, o: BuildOptions): HTMLElement {
  return el(
    'div',
    { class: 'rt', 'data-root': r.root },
    div(
      'top',
      r.kind === 'node' ? el('b', {}, nodeLink(r.root, o)) : el('b', {}, code(r.root)),
      chip('mute', r.state),
      r.waits_on_human ? chip('wait', 'waits on you') : '',
      chip('acc', `holds up ${r.weight}`),
    ),
    el(
      'ul',
      { class: 'holds' },
      ...r.holds.map((h) => el('li', { class: 'held', 'data-node': h.node }, nodeLink(h.node, o), h.via.length ? el('span', { class: 'via' }, ` via ${h.via.join(' › ')}`) : '')),
    ),
  );
}

function blockersPanel(impl: Implementation, o: BuildOptions): HTMLElement {
  const roots = impl.blockers.roots;
  return div('card sec blockers', el('h3', { class: 'h' }, 'Blockers'), ...(roots.length ? roots.map((r) => rootCard(r, o)) : [div('note', 'Nothing is blocked.')]));
}

export function renderImplementation(impl: Implementation, snap: Snapshot, o: BuildOptions): HTMLElement {
  const t = impl.flight.totals;
  const stat = (key: string, v: number, l: string, detail = '') =>
    el('div', { class: 'card stat', 'data-stat': key }, el('div', { class: 'v' }, String(v)), div('l', l), detail ? div('note', detail) : '');
  const switcher = div(
    'switch',
    ...(['flight', 'blockers'] as Switch[]).map((s) => {
      const b = el('button', { type: 'button', class: s === o.show ? 'on' : '' }, s === 'flight' ? 'In flight' : 'Blockers');
      b.addEventListener('click', () => o.onShow(s));
      return b;
    }),
  );
  return div(
    'impl',
    div('crumb', 'Implementation'),
    div('head', div('', el('h1', {}, 'Implementation'), div('meta', `at ${impl.sha.slice(0, 8)}`, impl.coverage.state === 'indexing' ? chip('wait', 'coverage indexing') : ''))),
    div(
      'stats',
      stat('pending', t.pending_nodes, 'Pending components', byState(t.by_state, STATE_ORDER)),
      stat('changes', t.live_changes, 'Live changes'),
      stat('requirements', t.requirements_pending, 'Requirements to do'),
    ),
    switcher,
    div('runs', o.show === 'blockers' ? blockersPanel(impl, o) : inFlight(impl, snap, o), renderActivity(impl.activity, o.now)),
  );
}

const ICON: Record<string, [string, string]> = {
  push: ['↑', 'accent'],
  claim: ['⚑', 'sky'],
  land: ['✓', 'green'],
  approval: ['◆', 'green'],
};

/** The feed: pushes and claims from the repo's events, lands and approvals from the
 * ledger, each naming its branch (or the item claimed), newest first. */
export function renderActivity(activity: Activity[], now: number): HTMLElement {
  const sorted = [...activity].sort((a, b) => b.at - a.at);
  const text = (a: Activity): (Node | string)[] => {
    switch (a.kind) {
      case 'push':
        return ['Pushed to ', code(a.ref)];
      case 'claim':
        return [`Claimed ${a.ref}`];
      case 'land':
        return [`${a.actor} landed `, code(a.ref)];
      case 'approval':
        return [`${a.actor} approved `, code(a.ref)];
      default:
        return [`${a.kind} `, code(a.ref)];
    }
  };
  return div(
    'feed card',
    div(
      'sec',
      el('h3', { class: 'h' }, 'Activity'),
      ...(sorted.length
        ? sorted.map((a) => {
            const [icon, colour] = ICON[a.kind] ?? ['•', 'muted'];
            return el(
              'div',
              { class: 'ev', 'data-kind': a.kind },
              el('div', { class: 'ic', style: `background:var(--${colour}-bg, var(--surface2));color:var(--${colour})` }, icon),
              div('', div('tx', ...text(a)), div('tm', `${ago(a.at, now)}${a.actor ? ` · ${a.actor}` : ''}`)),
            );
          })
        : [div('note', 'No recent activity.')]),
    ),
  );
}
