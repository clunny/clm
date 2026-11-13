// What a selected node shows: its contract, dependencies, changes in open PRs, work in
// flight with its pending requirements, and backlog items. A bottom sheet on narrow screens.

import { chip, div, el } from './dom';
import { proven } from './graph';
import { md } from './markdown';
import type { Implementation, NodeChanges, Review, Snapshot } from './types';

function prChanges(changed: NodeChanges): HTMLElement[] {
  return [
    ...(['contract', 'design', 'frontmatter'] as const)
      .filter((k) => changed[k])
      .map((k) => div('mini', el('b', {}, k, chip('modified', 'modified')))),
    ...(['added', 'modified', 'removed'] as const).flatMap((k) =>
      [...changed.requirements[k], ...changed.assets[k]].map((r) => div('mini', el('b', {}, r.id ?? `assets/${r.path}`, chip(k, k)))),
    ),
  ];
}

/** `impl` is null until the implementation state arrives; the graph doesn't wait for it.
 * `prs`: the reviews of the open spec PRs, as far as they're loaded. */
export function renderInspector(
  snap: Snapshot,
  impl: Implementation | null,
  prs: Review[],
  id: string,
  onClose?: () => void,
  /** The in-app link to a backlog item's page; without it an item is plain text. */
  itemHref?: (slug: string) => string,
): HTMLElement {
  const n = snap.nodes.find((x) => x.id === id);
  const s = snap.spec.nodes.find((x) => x.node === id);
  const sec = (title: string, cls: string, ...body: (Node | string)[]) =>
    div(`sec ${cls}`, el('h3', { class: 'h' }, title), ...body);
  const none = (text: string) => div('note', text);

  const head = div(
    'sec',
    div(
      'top',
      el('b', { style: 'font-size:16px' }, id),
      ' ',
      s ? chip(s.status === 'active' ? 'ok' : 'sky', `${s.status} · ${proven(snap, s)}`) : '',
    ),
    n ? div('note', n.title) : '',
    div('deps', ...(n?.depends_on ?? []).map((d) => el('span', { class: 'pill' }, d))),
    s?.owners.length ? div('note', `Owned by ${s.owners.join(' ')}`) : '',
  );

  const inPrs = prs.flatMap((pr) => {
    const changed = pr.nodes.find((c) => c.id === id);
    return changed ? [div('pr-changes', div('note', pr.branch), ...prChanges(changed))] : [];
  });
  const changes = snap.changes.filter((c) => c.nodes.includes(id));
  const backlog = snap.backlog.filter((b) => b.nodes.includes(id));

  const aside = el(
    'aside',
    { class: 'insp card' },
    div('grab'),
    onClose ? el('button', { type: 'button', class: 'close', 'aria-label': 'Close', onclick: onClose as EventListener }, '×') : '',
    head,
    sec('In open PRs', 'inpr', ...(inPrs.length ? inPrs : [none('No open PR changes it.')])),
    sec('Contract', 'contract', n ? md(n.contract) : none('Not in this snapshot.')),
    sec(
      'Changes',
      'changes',
      ...(changes.length
        ? changes.map((c) =>
            el(
              'div',
              { class: 'mini', 'data-change': c.id },
              el('b', {}, c.id, chip(c.complete ? 'ok' : 'mute', c.complete ? 'complete' : c.status)),
              c.current !== null ? `${c.current}/${c.total} requirements current` : 'coverage indexing',
            ),
          )
        : [none('No change touches it.')]),
    ),
    sec('Work in flight', 'work', work(impl, id)),
    sec(
      'Backlog',
      'backlog',
      ...(backlog.length
        ? backlog.map((b) => {
            const name = el('b', {}, b.slug, chip('mute', b.kind));
            return itemHref ? el('a', { class: 'mini', href: itemHref(b.slug) }, name, b.title) : div('mini', name, b.title);
          })
        : [none('None.')]),
    ),
  );
  if (onClose) closeOnEscape(aside, onClose);
  return aside;
}

/** Closes `aside` on Escape for as long as it is on the page: a listener whose inspector
 * has been replaced or removed takes itself off at the next key. */
function closeOnEscape(aside: HTMLElement, onClose: () => void) {
  const onKey = (e: KeyboardEvent) => {
    if (!aside.isConnected) return document.removeEventListener('keydown', onKey);
    if (e.key === 'Escape') onClose();
  };
  document.addEventListener('keydown', onKey);
}

/** Node `id`'s queue state and pending count as the `flight` model gives them, and each
 * pending requirement with its state, an obsolete one with the markers that still cite it
 * (web/inspector-pending). While coverage is indexing every requirement would read
 * `unbuilt`, so none is listed. */
function work(impl: Implementation | null, id: string): HTMLElement {
  if (impl === null) return div('note', 'Loading work in flight…');
  if (impl.coverage.state === 'indexing') return div('note', 'Coverage is indexing: pending requirements show once it completes.');
  const f = impl.flight.nodes.find((x) => x.node === id);
  if (!f || f.pending.length === 0) return div('note', 'Nothing pending.');
  return div(
    '',
    div('mini', el('b', {}, f.node, chip('mute', f.state)), `${f.pending.length} pending`),
    el(
      'ul',
      { class: 'reqs' },
      ...f.pending.map((p) =>
        el('li', { class: `rq ${p.state}` }, el('code', {}, p.id), ` ${p.state}`, p.state === 'obsolete' && p.markers.length ? ` · ${p.markers.join(', ')}` : ''),
      ),
    ),
  );
}
