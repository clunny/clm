// The Capabilities view: every capability's cost, ranked, as a table from 900 px and cards
// below (web/capabilities-list), a capability's page in `clm capabilities <slug>`'s order
// (web/capability-page), and its decision (web/decide-buttons). The Worker's report has no
// history, so the numbers only git gives are left to `clm capabilities <slug>`. The
// Architecture view (architecture.ts) draws its pages with the same pieces.

import { SignedOut } from './api';
import { Child, chip, code, div, el } from './dom';
import type { Capabilities, Cost, Decided, DecisionKind, Flag, Rating } from './types';

/** `850`, `85k` or `1.3M`, as `clm capabilities` prints them. */
export function tokens(n: number): string {
  if (n >= 999_500) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 999.5) return `${Math.round(n / 1e3)}k`;
  return String(Math.round(n));
}

/** `$` and the dollars: to the cent under $10, to the tenth above, as `clm` prints them. */
export function usd(dollars: number): string {
  const cents = Math.round(dollars * 100) / 100;
  return cents < 10 ? `$${cents.toFixed(2)}` : `$${dollars.toFixed(1)}`;
}

/** What a capability and an architectural constraint both show. */
type Reviewed = Pick<Cost, 'rules' | 'flags' | 'overlap' | 'decisions'>;

const day = (at: string) => at.slice(0, 10);
export const unsettled = (c: Reviewed) => c.flags.filter((f) => !f.settled).length;
const short = (id: string) => id.replace(/^(capability|architecture)\//, '');

/** Its latest decision, or `never decided`. */
export function latest(c: Reviewed): string {
  const d = c.decisions[c.decisions.length - 1];
  return d ? `${d.decision} ${d.value} ${day(d.at)}` : 'never decided';
}

/** "Live without?" in a cell: how many of its rules serve only it, and what the rest serve. */
export function without(c: Reviewed): string {
  const o = c.overlap;
  if (!o.shared.length) return 'all its rules only here';
  return `${o.only} of ${c.rules.length} rules only here; also ${o.shared.map(([id, k]) => `${short(id)} (${k})`).join(', ')}`;
}

export interface ListOptions {
  /** Show only the capabilities with an unsettled flag. */
  flaggedOnly: boolean;
  onFilter: (flaggedOnly: boolean) => void;
  /** The in-app link to a capability's page. */
  link: (slug: string) => string;
}

export function renderCapabilities(data: Capabilities, o: ListOptions): HTMLElement {
  const all = data.report.capabilities;
  const shown = o.flaggedOnly ? all.filter((c) => c.flagged) : all;
  const top = Math.max(...all.map((c) => c.spend.dollars), 0);
  const flagged = all.filter((c) => c.flagged).length;
  const filter = el('button', { type: 'button', class: o.flaggedOnly ? 'btn flag-only on' : 'btn flag-only', 'aria-pressed': String(o.flaggedOnly) }, '⚑ only');
  filter.addEventListener('click', () => o.onFilter(!o.flaggedOnly));
  const name = (c: Cost) => [el('b', {}, c.slug), c.flagged ? ' ⚑' : ''];
  const spend = (c: Cost) => `≈${usd(c.spend.dollars)} · ${tokens(c.spend.work_tokens)} tok · ${c.approvals} reviews`;
  const bar = (c: Cost) => div(c.flagged ? 'cbar flag' : 'cbar', el('i', { style: `width:${top > 0 ? Math.max(2, Math.round((c.spend.dollars / top) * 100)) : 0}%` }));
  const table = el(
    'table',
    { class: 'caps' },
    el('thead', {}, el('tr', {}, ...['Capability', 'Cost', 'Workarounds', 'Live without?', 'Decided'].map((h) => el('th', {}, h)))),
    el(
      'tbody',
      {},
      ...shown.map((c) =>
        el(
          'tr',
          { class: c.flagged ? 'cap flag' : 'cap', 'data-slug': c.slug },
          el('td', {}, el('a', { href: o.link(c.slug) }, ...name(c)), div('note', c.statement)),
          el('td', {}, bar(c), spend(c)),
          el('td', {}, String(unsettled(c))),
          el('td', {}, without(c)),
          el('td', {}, latest(c)),
        ),
      ),
    ),
  );
  const cards = div(
    'ccards',
    ...shown.map((c) =>
      el(
        'a',
        { class: c.flagged ? 'ccard card flag' : 'ccard card', href: o.link(c.slug), 'data-slug': c.slug },
        div('row', el('span', {}, ...name(c)), el('small', {}, `${c.rules.length} rules`)),
        div('note', c.statement),
        bar(c),
        div('row note', el('span', {}, spend(c)), el('span', {}, `${unsettled(c)} workarounds`)),
        div('note', `Live without? ${without(c)} · ${latest(c)}`),
      ),
    ),
  );
  const r = data.report;
  return div(
    'capabilities',
    div('crumb', 'Capabilities'),
    div(
      'head',
      div(
        '',
        el('h1', {}, 'Capabilities'),
        div('meta', `${all.length} · ${flagged} ⚑ · costs are estimates: a component's cost is split by its rules' share`),
      ),
      filter,
    ),
    shown.length ? [table, cards] : div('empty', all.length ? 'No capability is flagged.' : 'No capabilities: CAPABILITIES.md holds none.'),
    div('note', `Unattributed ≈${usd(r.unattributed.dollars)} · architecture only ≈${usd(r.architecture_only.dollars)}. Commits, questions answered, blocking and sizes need git: run clm capabilities <slug>.`),
  );
}

/** What the page says after a decision. */
export interface Note {
  text: string;
  /** The command a signed repository asks for, from the Worker's 422. */
  command?: string;
}

export interface PageOptions {
  offline: boolean;
  /** A decision from this page is in flight: the buttons stay disabled until it ends. */
  busy: boolean;
  /** The value chosen, kept by the caller across redraws. */
  value: Rating | null;
  onValue: (v: Rating) => void;
  onDecide: (d: DecisionKind) => void;
  /** The link back to the list. */
  back: string;
  note: Note | null;
}

const VALUES: Rating[] = ['high', 'medium', 'low'];
const DECISIONS: [DecisionKind, string][] = [
  ['keep', 'Keep'],
  ['cut', 'Cut'],
  ['redesign', 'Redesign'],
  ['accept', 'Accept as is'],
];

export function section(sec: string, title: string, ...body: Child[]): HTMLElement {
  return el('section', { class: 'csec', 'data-sec': sec }, el('h2', {}, title), ...body);
}

/** The workarounds section: each flag a `<details>` whose summary is its title and whose
 * body is its text. */
export function workarounds(flags: Flag[]): HTMLElement {
  const open = flags.filter((f) => !f.settled).length;
  return section(
    'workarounds',
    `Workarounds (${flags.length} open, ${open} unsettled)`,
    flags.length
      ? flags.map((f) =>
          el(
            'details',
            { class: f.settled ? 'flagd settled' : 'flagd' },
            el('summary', {}, f.title),
            div('note', f.settled ? 'settled · ' : '⚑ unsettled · ', code(f.requirement), ' · ', f.slug),
            ...f.text
              .split(/\n\s*\n/)
              .map((p) => p.trim())
              .filter(Boolean)
              .map((p) => el('p', {}, p)),
          ),
        )
      : el('p', { class: 'note' }, 'None open.'),
  );
}

/** "Can we live without it?", from its overlap. */
export function withoutSection(c: Reviewed): HTMLElement {
  const o = c.overlap;
  return section(
    'without',
    'Can we live without it?',
    el(
      'p',
      {},
      o.shared.length
        ? `${o.only} of its ${c.rules.length} rules serve only it; the rest also serve ${o.shared.map(([id, k]) => `${short(id)} (${k})`).join(', ')}`
        : 'All its rules serve only it.',
    ),
  );
}

export function decisionsSection(decisions: Decided[]): HTMLElement {
  return section(
    'decisions',
    'Decisions',
    decisions.length
      ? el('ul', {}, ...decisions.map((d) => el('li', {}, `${day(d.at)} ${d.decision} (value ${d.value}) by ${d.writer}`)))
      : el('p', { class: 'note' }, 'never decided'),
  );
}

/** The `clm` command that decides on `slug` of `layer` from a clone, as a signed repository
 * asks: `clm capabilities decide` or `clm architecture decide`. */
export const decideCommand = (layer: Layer, slug: string) => `clm ${layer} decide ${slug} --value high|medium|low --decision keep|cut|redesign|accept`;

/** The value and decision buttons, in a 2×2 grid below 900 px (web/decide-buttons,
 * web/architecture-decide-buttons), for `slug` of `layer`. */
export function decideBar(o: PageOptions, signed: boolean, layer: Layer, slug: string): HTMLElement {
  const command = decideCommand(layer, slug);
  const why = o.busy ? 'Deciding…' : !o.value ? 'Choose a value first' : o.offline ? 'Offline' : signed ? `This repository counts only signed records: decide with clm ${layer} decide` : null;
  const values = VALUES.map((v) => {
    const b = el('button', { type: 'button', class: v === o.value ? 'btn value on' : 'btn value', 'aria-pressed': String(v === o.value) }, v);
    b.addEventListener('click', () => o.onValue(v));
    return b;
  });
  const buttons = DECISIONS.map(([d, label]) => el('button', { type: 'button', class: 'btn decide', 'data-decision': d, disabled: why !== null, title: why ?? `${label}, value ${o.value}` }, label));
  // One request per tap, and none while one is in flight: the page is drawn again after it.
  buttons.forEach((b, i) =>
    b.addEventListener('click', () => {
      if (buttons.some((x) => x.disabled)) return;
      for (const x of buttons) x.disabled = true;
      o.onDecide(DECISIONS[i][0]);
    }),
  );
  return div(
    'decide-bar',
    div('values', el('span', { class: 'note' }, 'Value'), ...values),
    div('decides', ...buttons),
    // A 422 signed's note gives the Worker's command, which says more than this one.
    signed && !o.note?.command ? div('note signed-note', "This repository counts only records signed with the reviewer's SSH key. Decide from your clone with ", code(command)) : '',
    o.note ? div('note decide-note', o.note.text, o.note.command ? [' Decide from your clone with ', code(o.note.command)] : '') : '',
  );
}

export function renderCapability(data: Capabilities, slug: string, o: PageOptions): HTMLElement {
  const back = el('a', { class: 'back', href: o.back }, '‹ Capabilities');
  const c = data.report.capabilities.find((x) => x.slug === slug);
  if (!c) return div('capage', back, div('empty', `The report holds no capability ${slug}.`));
  const s = c.spend;
  return div(
    'capage',
    back,
    div('head', div('', el('h1', {}, `${c.slug}${c.flagged ? ' ⚑' : ''}`), div('meta', `${c.rules.length} rules`, ...c.nodes.map((n) => chip('node', n))))),
    el('p', { class: 'statement' }, c.statement),
    section('why', 'Why we need it', el('p', {}, c.why)),
    section(
      'cost',
      'What it costs',
      el(
        'ul',
        {},
        el('li', {}, `≈${usd(s.dollars)} · ${tokens(s.work_tokens)} tokens · ${(s.active_seconds / 3600).toFixed(1)}h active · ${tokens(s.cache_read_tokens)} cache reads`),
        el('li', {}, `${c.changes} changes · ${c.approvals} approvals · ${c.flags_settled} flags settled`),
        el('li', {}, `Declared burdens: ${c.burdens.length ? c.burdens.join(', ') : 'none'}`),
      ),
      div('note', 'An estimate: a component\'s cost is split by its rules\' share. For its commits, questions answered, blocking and size, run ', code(`clm capabilities ${c.slug}`), '.'),
    ),
    workarounds(c.flags),
    withoutSection(c),
    section('rules', 'Rules', el('details', {}, el('summary', {}, `${c.rules.length} rules`), el('ul', {}, ...c.rules.map((r) => el('li', {}, code(r)))))),
    decisionsSection(c.decisions),
    decideBar(o, data.signed, 'capabilities', c.slug),
  );
}

export interface Deciding {
  post: (path: string, body: unknown) => Promise<Response>;
  /** Called when the Worker says the session has ended. */
  signedOut?: () => void;
  /** Loads the report again from the Worker: false when it couldn't. */
  reload: () => Promise<boolean>;
}

/** What a decision is on: a capability, or an architectural constraint, by its route. */
export type Layer = 'capabilities' | 'architecture';

/** Posts a decision on `slug` of `layer` at `version` (web.worker/decide,
 * web.worker/architecture-decide), then loads the report again, and says how it went. */
export async function decide(base: string, layer: Layer, slug: string, version: string, value: Rating, decision: DecisionKind, d: Deciding): Promise<Note> {
  let res: Response;
  try {
    res = await d.post(`${base}/${layer}/${encodeURIComponent(slug)}/decide`, { version, value, decision });
  } catch (e) {
    if (e instanceof SignedOut) d.signedOut?.();
    return { text: e instanceof SignedOut ? 'Signed out. Sign in again to decide.' : 'Offline: decide when back online.' };
  }
  const body = res.status === 422 ? await res.json().catch(() => null) : null;
  const signed = body?.error === 'signed';
  const what = layer === 'capabilities' ? 'capability' : 'entry';
  let note: Note;
  if (res.ok) note = { text: `Decided: ${decision}, value ${value}.` };
  else if (res.status === 409) note = { text: `The ${what} changed since it was shown: here it is as it is now. Read it again before deciding.` };
  else if (signed) note = { text: 'Not decided: this repository counts only records signed with your SSH key.', command: typeof body.command === 'string' ? body.command : decideCommand(layer, slug) };
  // A 422 may mean the entry left the tip, or its specs don't load: draw it as it is now.
  else if (res.status === 422) note = { text: `Not decided (422): the Worker refused the ${typeof body?.error === 'string' ? body.error : 'request'}.` };
  else return { text: `Not decided (${res.status}).` };
  if (!(await d.reload())) note = { ...note, text: `${note.text} Its report could not be loaded again: Reload.` };
  return note;
}
