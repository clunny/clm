// The backlog: "Waiting on you" first (question and human-task items, with the requirements
// each blocks, `requirement-blocks`), then the other items in columns by kind, filterable
// by component (web.views/backlog-view), each item marked with what holds it (web/backlog-taken), and
// New proposal with the open proposals under Proposed (web/propose-from-backlog).

import { answerButtons, type Answering } from './answers';
import { chip, div, el } from './dom';
import { Draft, drafts, person, sendOnce, submitDraft } from './forms';
import { md } from './markdown';
import { when } from './time';
import type { BacklogView, Proposal, Snapshot, Taken } from './types';

const WAITING = ['question', 'human-task'];
const COLUMNS: [string, string, string][] = [
  ['bug', 'Bugs', 'var(--rose)'],
  ['deferred', 'Deferred', 'var(--amber)'],
  ['idea', 'Ideas', 'var(--accent)'],
];

export interface Waiting {
  item: BacklogView;
  /** The requirements the item blocks. */
  blocks: string[];
}

export function waiting(snap: Snapshot): Waiting[] {
  return snap.backlog.filter((b) => WAITING.includes(b.kind)).map((item) => ({ item, blocks: item.blocks }));
}

/** An item's slug, whether named bare or as `backlog/<slug>`. */
const bare = (item: string) => item.replace(/^backlog\//, '');

/** What holds item `slug` (`propose-claims`): `resolved by <id>` when a change on main
 * names it, else `proposed in <id>` when proposed spec work does, else `claimed` when a
 * bare claim does; null when nothing holds it. */
export function takenMark(taken: Taken, slug: string): string | null {
  const by = (state: string) => taken.changes.find((c) => c.state === state && c.items.some((i) => bare(i) === slug));
  const main = by('main');
  if (main) return `resolved by ${main.id}`;
  const proposed = by('proposed');
  if (proposed) return `proposed in ${proposed.id}`;
  return taken.claims.some((c) => bare(c.item) === slug) ? 'claimed' : null;
}

/** The in-app link to a backlog item's page. */
type ItemLink = (slug: string) => string;

function card(b: BacklogView, taken: Taken | null, link?: ItemLink, blocks?: string[]): HTMLElement {
  const mark = taken ? takenMark(taken, b.slug) : null;
  return el(
    'div',
    { class: 'bi', 'data-slug': b.slug },
    link ? el('a', { class: 'ttl', href: link(b.slug) }, b.title) : div('ttl', b.title),
    ...b.nodes.map((n) => el('span', { class: 'pill' }, n)),
    mark ? chip('acc taken', mark) : '',
    blocks?.length ? div('blocks', `Blocks ${blocks.join(', ')}`) : '',
    div('slug', b.slug),
  );
}

export interface Proposing {
  /** The implementation state's open proposals; null until it arrives. */
  proposals: Proposal[] | null;
  offline: boolean;
  /** Sends a proposal: null once the Worker took it, else why not, for the form to show. */
  onPropose: (title: string, body: string) => Promise<string | null>;
  /** Withdraws the proposal `id`: null once the Worker took it, else why not. */
  onWithdraw: (id: string) => Promise<string | null>;
}

/** New proposal: a button that opens a form with a title and an intent, open at once while
 * `drafts` holds what was typed into it. It has no picker of components: the draft that
 * takes the item up decides them. */
function proposalForm(key: string, p: Proposing): HTMLElement {
  const wrap = div('fbw newprop');
  const disabled = p.offline ? 'Offline' : null;
  const toggle = el('button', { type: 'button', class: 'btn pri propose', disabled: disabled !== null, title: disabled ?? 'Propose an item for the backlog' }, 'New proposal');
  toggle.addEventListener('click', () => {
    if (toggle.disabled) return;
    drafts.set(key, { title: '', text: '' });
    draw();
  });
  const draw = () => {
    const draft: Draft | undefined = drafts.get(key);
    if (!draft) return wrap.replaceChildren(toggle);
    const title = el('input', { type: 'text', name: 'title', 'aria-label': 'Title', placeholder: 'Title', maxlength: 120, 'data-draft': `${key}|title` });
    title.value = draft.title ?? '';
    title.addEventListener('input', () => (draft.title = title.value));
    const body = el('textarea', { rows: 4, 'aria-label': 'Intent', placeholder: 'The intent: what and why (plain text or Markdown)', 'data-draft': `${key}|text` });
    body.value = draft.text;
    body.addEventListener('input', () => (draft.text = body.value));
    const send = el('button', { type: 'button', class: 'btn pri send', disabled: disabled !== null, title: disabled ?? 'Propose' }, 'Propose');
    const cancel = el('button', { type: 'button', class: 'btn cancel' }, 'Cancel');
    cancel.addEventListener('click', () => {
      drafts.delete(key);
      draw();
    });
    const err = div('err');
    sendOnce(
      send,
      err,
      () => {
        const t = title.value.trim();
        if (!t) return Promise.resolve('Give it a title first.');
        return submitDraft(key, draft, () => p.onPropose(t, body.value.trim()));
      },
      draw,
    );
    wrap.replaceChildren(
      div(
        'fbform proposal',
        title,
        body,
        div('fbact', send, cancel),
        err,
      ),
    );
  };
  draw();
  return wrap;
}

/** Proposed: the open proposals, each with Withdraw when it is the person's own. */
function proposedRow(list: Proposal[], p: Proposing): HTMLElement | '' {
  if (!list.length) return '';
  return div(
    'card sec proposed',
    el('h3', { class: 'h' }, 'Proposed ', chip('sky', String(list.length))),
    ...list.map((x) => {
      let withdraw: HTMLElement | '' = '';
      if (x.mine) {
        const w = el('button', { type: 'button', class: 'btn withdraw', disabled: p.offline, title: p.offline ? 'Offline' : 'Withdraw this proposal' }, 'Withdraw');
        const err = el('span', { class: 'err' });
        w.addEventListener('click', async () => {
          if (w.disabled) return;
          // One withdrawal per tap; on success the view is drawn again without it.
          w.disabled = true;
          err.textContent = '';
          const why = await p.onWithdraw(x.id).catch(() => 'Not withdrawn: offline.');
          if (why !== null) {
            err.textContent = why;
            w.disabled = false;
          }
        });
        withdraw = div('wd', w, ' ', err);
      }
      return el(
        'div',
        { class: 'bi prop', 'data-id': x.id },
        div('ttl', x.title),
        div('slug', `${person(x.writer)} · ${when(x.at)}`),
        div('note', 'The next build files it as a backlog item.'),
        withdraw,
      );
    }),
  );
}

/** The backlog, with each item's `taken` mark once the implementation state has arrived
 * (`taken` null until then), and New proposal and Proposed when `p` is given. */
export function renderBacklog(
  snap: Snapshot,
  taken: Taken | null,
  node: string | null,
  onFilter: (node: string | null) => void,
  p?: Proposing,
  link?: ItemLink,
): HTMLElement {
  const on = (b: BacklogView) => node === null || b.nodes.includes(node);
  const nodes = [...new Set(snap.backlog.flatMap((b) => b.nodes))].sort();
  const filter = div(
    'filter',
    el('span', { class: 'note', style: 'margin:0 6px 0 0' }, 'Component'),
    el('button', { type: 'button', class: node === null ? 'pill on' : 'pill', onclick: (() => onFilter(null)) as EventListener }, 'all'),
    ...nodes.map((n) =>
      el('button', { type: 'button', class: node === n ? 'pill on' : 'pill', 'data-node': n, onclick: (() => onFilter(n)) as EventListener }, n),
    ),
  );

  const wait = waiting(snap).filter((w) => on(w.item));
  const waitRow = wait.length
    ? div(
        'card sec waiting',
        el('h3', { class: 'h' }, 'Waiting on you ', chip('wait', String(wait.length))),
        div('kan', ...wait.map((w) => div('', chip(w.item.kind === 'human-task' ? 'wait' : 'sky', w.item.kind), card(w.item, taken, link, w.blocks)))),
      )
    : '';

  const rest = snap.backlog.filter((b) => !WAITING.includes(b.kind) && on(b));
  const known = COLUMNS.map((c) => c[0]);
  const other = [...new Set(rest.map((b) => b.kind).filter((k) => !known.includes(k)))];
  const cols = [...COLUMNS, ...other.map((k): [string, string, string] => [k, k, 'var(--muted)'])].map(([kind, label, colour]) => {
    const items = rest.filter((b) => b.kind === kind);
    return el(
      'div',
      { class: 'col', 'data-kind': kind },
      div('colh', el('span', { class: 'dot', style: `background:${colour};margin:0` }), label, el('span', { class: 'c' }, String(items.length))),
      ...items.map((b) => card(b, taken, link)),
    );
  });

  return div(
    'backlog',
    div('crumb', 'Backlog'),
    div(
      'head',
      div('', el('h1', {}, 'Backlog'), div('meta', `${snap.backlog.length} items across ${nodes.length} components`)),
      filter,
    ),
    p ? proposalForm(`${snap.repo}|proposal`, p) : '',
    waitRow,
    p ? proposedRow(p.proposals ?? [], p) : '',
    div('kan', ...cols),
  );
}

/** A backlog item's page at `#/<owner>/<repo>/backlog/<slug>`: what the snapshot holds of
 * it, and what holds it once `taken` has arrived; a slug the backlog doesn't hold says so,
 * with a link to the Backlog (`back`). */
export function renderBacklogItem(snap: Snapshot, slug: string, taken: Taken | null, back: string, answering?: Answering): HTMLElement {
  const b = snap.backlog.find((x) => x.slug === slug);
  const crumb = el('a', { class: 'back', href: back }, '‹ Backlog');
  if (!b) return div('bipage', crumb, div('empty', `The item ${slug} isn't in the backlog. `, el('a', { href: back }, 'Back to the Backlog')));
  const mark = taken ? takenMark(taken, b.slug) : null;
  const given = (answering?.answers ?? []).filter((x) => x.item === b.slug);
  return div(
    'bipage',
    crumb,
    el('h1', {}, b.title),
    div('meta', chip('mute', b.kind), b.priority ? chip('mute prio', b.priority) : '', ...b.nodes.map((n) => chip('node', n)), mark ? chip('acc taken', mark) : ''),
    div('slug', b.slug),
    b.origin ? div('note', `Origin: ${b.origin}`) : '',
    b.blocks.length ? div('blocks', `Blocks ${b.blocks.join(', ')}`) : '',
    given.length
      ? div('given', el('h3', { class: 'h' }, 'Answers'), ...given.map((x) => div('note given-answer', el('strong', {}, x.answer), ` by ${person(x.writer)} · ${when(x.at)}`)))
      : '',
    answering ? answerButtons(b, answering) : '',
    b.body ? md(b.body) : '',
  );
}
