import { describe, expect, it, vi } from 'vitest';
import { renderArchitecture, renderConstraint } from './architecture';
import { decide } from './capabilities';
import type { ListOptions, PageOptions } from './capabilities';
import { architecture, constraint } from './fixtures';

const list = (o: Partial<ListOptions> = {}): ListOptions => ({ flaggedOnly: false, onFilter: vi.fn(), link: (s) => `#/o/a/architecture/${s}`, ...o });
const page = (o: Partial<PageOptions> = {}): PageOptions => ({ offline: false, busy: false, value: 'medium', onValue: vi.fn(), onDecide: vi.fn(), back: '#/o/a/architecture', note: null, ...o });
const slugs = (e: HTMLElement, sel: string) => [...e.querySelectorAll<HTMLElement>(sel)].map((r) => r.dataset.slug);
const decideButtons = (e: HTMLElement) => [...e.querySelectorAll<HTMLButtonElement>('button.decide')];
const proof = (e: HTMLElement, slug: string) => e.querySelector<HTMLElement>(`tr.cap[data-slug=${slug}] .proof`)!;

// req:web.views/architecture-list@d644a9d4
describe('architecture list', () => {
  it('ranked, held and flagged: the report order, as a table and as cards, held 1/2 in amber, untested alone', () => {
    const view = renderArchitecture(architecture(), list());
    expect(slugs(view, 'tr.cap')).toEqual(['e', 'g']);
    expect(slugs(view, '.ccard')).toEqual(['e', 'g']);
    const e = view.querySelector<HTMLElement>('tr.cap[data-slug=e]')!;
    const g = view.querySelector<HTMLElement>('tr.cap[data-slug=g]')!;
    expect(e.textContent).toContain('⚑');
    expect(g.textContent).not.toContain('⚑');
    expect(e.textContent).toContain('CLM SHALL e.');
    expect(proof(view, 'e').textContent).toBe('held 1/2 + untested');
    expect(proof(view, 'e').classList.contains('short')).toBe(true);
    expect(proof(view, 'g').textContent).toBe('untested');
    expect(proof(view, 'g').classList.contains('short')).toBe(false);
    expect(e.textContent).toContain('≈$5.00');
    expect(e.textContent).toContain('500k tok');
    expect(e.textContent).toContain('1 reviews');
    expect(e.textContent).toContain('1 of 2 rules only here; also other (1)');
    expect(e.textContent).toContain('never decided');
    // The card holds the same facts.
    const card = view.querySelector<HTMLElement>('.ccard[data-slug=e]')!;
    expect(card.querySelector('.proof')!.textContent).toBe('held 1/2 + untested');
    expect(card.getAttribute('href')).toBe('#/o/a/architecture/e');
    expect(view.querySelector<HTMLAnchorElement>('tr.cap[data-slug=e] a')!.getAttribute('href')).toBe('#/o/a/architecture/e');
    expect(view.querySelector('.cost-note')!.textContent).toContain('counted apart from the capabilities');
  });

  it('fully held: held 2/2 is not amber, and an entry with no rule and no untested text reads held 0/0', () => {
    const a = architecture();
    a.architecture = [
      constraint('h', 1, { enforced_by: [{ rule: 'a/x', current: true }, { rule: 'a/z', current: true }] }),
      constraint('z', 0),
    ];
    const view = renderArchitecture(a, list());
    expect(proof(view, 'h').textContent).toBe('held 2/2');
    expect(proof(view, 'h').classList.contains('short')).toBe(false);
    expect(proof(view, 'z').textContent).toBe('held 0/0');
  });

  it('flag filter: ⚑ only lists the flagged ones', () => {
    const o = list();
    renderArchitecture(architecture(), o).querySelector<HTMLButtonElement>('button.flag-only')!.click();
    expect(o.onFilter).toHaveBeenCalledWith(true);
    const only = renderArchitecture(architecture(), list({ flaggedOnly: true }));
    expect(slugs(only, 'tr.cap')).toEqual(['e']);
    expect(slugs(only, '.ccard')).toEqual(['e']);
    expect(only.querySelector('button.flag-only')!.getAttribute('aria-pressed')).toBe('true');
  });

  it('indexing: the proof reads ? / n and the note says coverage is still indexing', () => {
    const view = renderArchitecture(architecture({ indexing: true }), list());
    expect(proof(view, 'e').textContent).toBe('? / 2 + untested');
    expect(view.querySelector('.cost-note')!.textContent).toContain('still indexing');
    expect(renderArchitecture(architecture(), list()).querySelector('.cost-note')!.textContent).not.toContain('indexing');
  });

});

// req:web/architecture-page@d01678bd
describe('architecture page', () => {
  it('sections in order: statement, held with each rule and its untested text, cost, workarounds, without, decisions, then the buttons', () => {
    const view = renderConstraint(architecture(), 'e', page());
    const secs = [...view.querySelectorAll<HTMLElement>('section[data-sec]')].map((s) => s.dataset.sec);
    expect(secs).toEqual(['held', 'cost', 'workarounds', 'without', 'decisions']);
    expect(view.querySelector('.statement')!.textContent).toBe('CLM SHALL e.');
    const held = view.querySelector('section[data-sec=held]')!;
    const rules = [...held.querySelectorAll<HTMLElement>('li.rule')].map((r) => [r.dataset.rule, r.dataset.state]);
    expect(rules).toEqual([
      ['a/x', 'current'],
      ['a/y', 'pending'],
    ]);
    expect(held.querySelector('li.rule[data-rule="a/x"] .chip')!.classList.contains('ok')).toBe(true);
    expect(held.querySelector('li.rule[data-rule="a/y"] .chip')!.classList.contains('wait')).toBe(true);
    const untested = held.querySelector('.untested')!;
    expect(untested.textContent).toContain('Partly judgement.');
    expect(held.querySelector('ul')!.compareDocumentPosition(untested) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(view.querySelector('section[data-sec=cost]')!.textContent).toContain('counted apart from the capabilities');
    const d = view.querySelector<HTMLDetailsElement>('section[data-sec=workarounds] details')!;
    expect(d.querySelector('summary')!.textContent).toBe('Gate misfires');
    expect(d.textContent).toContain('The gate misfires on renames.');
    expect(view.querySelector('section[data-sec=without]')!.textContent).toContain('1 of its 2 rules serve only it');
    expect(view.querySelector('section[data-sec=decisions]')!.textContent).toContain('never decided');
    const last = view.querySelector('section[data-sec=decisions]')!;
    expect(last.compareDocumentPosition(view.querySelector('.decide-bar')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(view.querySelector<HTMLAnchorElement>('a.back')!.getAttribute('href')).toBe('#/o/a/architecture');
  });

  it('indexing: every rule is drawn unknown', () => {
    const a = architecture({ indexing: true });
    a.architecture[0].enforced_by = a.architecture[0].enforced_by.map((r) => ({ ...r, current: null }));
    const held = renderConstraint(a, 'e', page()).querySelector('section[data-sec=held]')!;
    expect([...held.querySelectorAll<HTMLElement>('li.rule')].map((r) => r.dataset.state)).toEqual(['unknown', 'unknown']);
    expect(held.textContent).toContain('indexing');
    // Indexing alone makes a rule unknown, whatever its `current` says.
    const known = renderConstraint(architecture({ indexing: true }), 'e', page()).querySelector('section[data-sec=held]')!;
    expect([...known.querySelectorAll<HTMLElement>('li.rule')].map((r) => r.dataset.state)).toEqual(['unknown', 'unknown']);
  });

  it('an entry without rules or untested text, and an unknown slug', () => {
    const held = renderConstraint(architecture(), 'g', page()).querySelector('section[data-sec=held]')!;
    expect(held.querySelectorAll('li.rule')).toHaveLength(0);
    expect(held.textContent).toContain('A judgement.');
    expect(renderConstraint(architecture(), 'zzz', page()).textContent).toContain('zzz');
  });
});

// req:web.views/architecture-decide-buttons@41e20cf5
describe('architecture decide', () => {
  it('one request per tap: a second tap while the first is in flight sends nothing', () => {
    const o = page();
    const view = renderConstraint(architecture(), 'e', o);
    expect(decideButtons(view).map((b) => b.textContent)).toEqual(['Keep', 'Cut', 'Redesign', 'Accept as is']);
    const redesign = decideButtons(view)[2];
    redesign.click();
    redesign.click();
    expect(o.onDecide).toHaveBeenCalledTimes(1);
    expect(o.onDecide).toHaveBeenCalledWith('redesign');
    expect(decideButtons(view).every((b) => b.disabled)).toBe(true);
  });

  it('signed: the buttons are disabled and the note gives clm architecture decide', () => {
    for (const [a, o] of [
      [architecture(), page({ offline: true })],
      [architecture(), page({ value: null })],
      [architecture({ signed: true }), page()],
    ] as const) {
      expect(decideButtons(renderConstraint(a, 'e', o)).every((b) => b.disabled)).toBe(true);
    }
    expect(decideButtons(renderConstraint(architecture(), 'e', page())).every((b) => !b.disabled)).toBe(true);
    const signed = renderConstraint(architecture({ signed: true }), 'e', page());
    expect(signed.querySelector('.signed-note code')!.textContent).toBe('clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept');
    expect(renderConstraint(architecture(), 'e', page()).querySelector('.signed-note')).toBeNull();
  });

  const answer = (status: number, body?: unknown) => new Response(body === undefined ? '' : JSON.stringify(body), { status });

  it('posts the version shown, the value and the decision to the architecture route, then loads the report again', async () => {
    const post = vi.fn(async () => answer(200, {}));
    const reload = vi.fn(async () => true);
    const note = await decide('/api/repos/o/a', 'architecture', 'e', 'v', 'medium', 'redesign', { post, reload });
    expect(post).toHaveBeenCalledTimes(1);
    expect(post).toHaveBeenCalledWith('/api/repos/o/a/architecture/e/decide', { version: 'v', value: 'medium', decision: 'redesign' });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(note.text).toContain('Decided');
  });

  it('a 409 says the entry changed and loads the report again; a 422 signed gives the Worker’s command', async () => {
    const reload = vi.fn(async () => true);
    const moved = await decide('/b', 'architecture', 'e', 'v', 'low', 'cut', { post: async () => answer(409), reload });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(moved.text).toContain('The entry changed since it was shown');
    const command = 'clm architecture decide e --value low --decision cut';
    const signed = await decide('/b', 'architecture', 'e', 'v', 'low', 'cut', { post: async () => answer(422, { error: 'signed', command }), reload: vi.fn(async () => true) });
    expect(signed.command).toBe(command);
    // Without the Worker's command, the note still gives one.
    const bare = await decide('/b', 'architecture', 'e', 'v', 'low', 'cut', { post: async () => answer(422, { error: 'signed' }), reload: vi.fn(async () => false) });
    expect(bare.command).toBe('clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept');
  });
});
