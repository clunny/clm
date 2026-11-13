// The Architecture view: every architectural constraint, whether it is held and what it
// costs, ranked, as a table from 900 px and cards below (web/architecture-list), an entry's
// page (web/architecture-page) and its decision (web/architecture-decide-buttons), drawn
// with the Capabilities view's pieces. Its cost is a second split, apart from the capabilities'.

import { ListOptions, PageOptions, decideBar, decisionsSection, latest, section, tokens, unsettled, usd, without, withoutSection, workarounds } from './capabilities';
import { chip, code, div, el } from './dom';
import { md } from './markdown';
import type { Architecture, Constraint, Enforcing } from './types';

/** An enforcing rule's state: unknown while the report is indexing. */
const state = (r: Enforcing, indexing: boolean): 'current' | 'pending' | 'unknown' => (indexing || r.current === null ? 'unknown' : r.current ? 'current' : 'pending');

/** Its proof: `held <c>/<n>`, `? / <n>` while indexing, ` + untested` after it when it also
 * holds `#### Untested`, or `untested` alone; `short` when a rule isn't current. */
function proof(c: Constraint, indexing: boolean): HTMLElement {
  const n = c.enforced_by.length;
  if (n === 0 && c.untested !== null) return el('span', { class: 'proof' }, 'untested');
  const held = c.enforced_by.filter((r) => r.current === true).length;
  const known = !indexing && c.enforced_by.every((r) => r.current !== null);
  const text = `${known ? `held ${held}/${n}` : `? / ${n}`}${c.untested !== null ? ' + untested' : ''}`;
  return el('span', { class: known && held < n ? 'proof short' : 'proof' }, text);
}

export function renderArchitecture(data: Architecture, o: ListOptions): HTMLElement {
  const all = data.architecture;
  const shown = o.flaggedOnly ? all.filter((c) => c.flagged) : all;
  const top = Math.max(...all.map((c) => c.spend.dollars), 0);
  const flagged = all.filter((c) => c.flagged).length;
  const filter = el('button', { type: 'button', class: o.flaggedOnly ? 'btn flag-only on' : 'btn flag-only', 'aria-pressed': String(o.flaggedOnly) }, '⚑ only');
  filter.addEventListener('click', () => o.onFilter(!o.flaggedOnly));
  const name = (c: Constraint) => [el('b', {}, c.slug), c.flagged ? ' ⚑' : ''];
  const spend = (c: Constraint) => `≈${usd(c.spend.dollars)} · ${tokens(c.spend.work_tokens)} tok · ${c.approvals} reviews`;
  const bar = (c: Constraint) => div(c.flagged ? 'cbar flag' : 'cbar', el('i', { style: `width:${top > 0 ? Math.max(2, Math.round((c.spend.dollars / top) * 100)) : 0}%` }));
  const table = el(
    'table',
    { class: 'caps' },
    el('thead', {}, el('tr', {}, ...['Requirement', 'Proof', 'Cost', 'Workarounds', 'Live without?', 'Decided'].map((h) => el('th', {}, h)))),
    el(
      'tbody',
      {},
      ...shown.map((c) =>
        el(
          'tr',
          { class: c.flagged ? 'cap flag' : 'cap', 'data-slug': c.slug },
          el('td', {}, el('a', { href: o.link(c.slug) }, ...name(c)), div('note', md(c.statement))),
          el('td', {}, proof(c, data.indexing)),
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
        div('row', el('span', {}, ...name(c)), proof(c, data.indexing)),
        div('note', md(c.statement)),
        bar(c),
        div('row note', el('span', {}, spend(c)), el('span', {}, `${unsettled(c)} workarounds`)),
        div('note', `Live without? ${without(c)} · ${latest(c)}`),
      ),
    ),
  );
  return div(
    'capabilities architecture',
    div('crumb', 'Architecture'),
    div('head', div('', el('h1', {}, 'Architecture'), div('meta', `${all.length} · ${flagged} ⚑`)), filter),
    div(
      'note cost-note',
      "An entry's cost is its enforcing rules' share, counted apart from the capabilities'. Costs are estimates.",
      data.indexing ? ' Coverage is still indexing: which rules hold is not known yet.' : '',
    ),
    shown.length ? [table, cards] : div('empty', all.length ? 'No architectural constraint is flagged.' : 'No architectural constraints: ARCHITECTURE.md holds none.'),
  );
}

export function renderConstraint(data: Architecture, slug: string, o: PageOptions): HTMLElement {
  const back = el('a', { class: 'back', href: o.back }, '‹ Architecture');
  const c = data.architecture.find((x) => x.slug === slug);
  if (!c) return div('capage', back, div('empty', `The report holds no architectural constraint ${slug}.`));
  const s = c.spend;
  const chips = { current: 'ok', pending: 'wait', unknown: 'mute' } as const;
  return div(
    'capage',
    back,
    div('head', div('', el('h1', {}, `${c.slug}${c.flagged ? ' ⚑' : ''}`), div('meta', `${c.rules.length} rules`, ...c.nodes.map((n) => chip('node', n))))),
    section('entry', 'The constraint', md(c.text)),
    section(
      'held',
      'Is it held?',
      data.indexing ? div('note', 'Coverage is still indexing: each rule’s state is unknown.') : '',
      c.enforced_by.length
        ? el(
            'ul',
            {},
            ...c.enforced_by.map((r) => {
              const st = state(r, data.indexing);
              return el('li', { class: 'rule', 'data-rule': r.rule, 'data-state': st }, code(r.rule), ' ', chip(chips[st], st));
            }),
          )
        : el('p', { class: 'note' }, 'No rule enforces it.'),
    ),
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
      div('note', "An estimate: its enforcing rules' share of their components' cost, counted apart from the capabilities'. For more, run ", code(`clm architecture ${c.slug}`), '.'),
    ),
    workarounds(c.flags),
    withoutSection(c),
    decisionsSection(c.decisions),
    decideBar(o, data.signed, 'architecture', c.slug),
  );
}
