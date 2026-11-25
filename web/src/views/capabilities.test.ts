import { describe, expect, it, vi } from 'vitest';
import { decide, renderCapabilities, renderCapability, tokens } from './capabilities';
import type { Deciding, ListOptions, Note, PageOptions } from './capabilities';
import type { Capabilities, DecisionKind } from './types';
import { capabilities, cost } from './fixtures';
import { SignedOut } from './api';

const list = (o: Partial<ListOptions> = {}): ListOptions => ({ flaggedOnly: false, onFilter: vi.fn(), link: (s) => `#/o/a/capabilities/${s}`, ...o });
const page = (o: Partial<PageOptions> = {}): PageOptions => ({ offline: false, busy: false, value: 'low', onValue: vi.fn(), onDecide: vi.fn(), back: '#/o/a/capabilities', note: null, ...o });
const slugs = (e: HTMLElement, sel: string) => [...e.querySelectorAll<HTMLElement>(sel)].map((r) => r.dataset.slug);
const decideButtons = (e: HTMLElement) => [...e.querySelectorAll<HTMLButtonElement>('button.decide')];

describe('capabilities list', () => {
  // req:web.views/capabilities-list@d74a43a5
  it('list ranked and flagged: the report order, as a table and as cards, only a marked ⚑', () => {
    const view = renderCapabilities(capabilities(), list());
    expect(slugs(view, 'tr.cap')).toEqual(['b', 'a']);
    expect(slugs(view, '.ccard')).toEqual(['b', 'a']);
    const a = view.querySelector<HTMLElement>('tr.cap[data-slug=a]')!;
    const b = view.querySelector<HTMLElement>('tr.cap[data-slug=b]')!;
    expect(a.textContent).toContain('⚑');
    expect(b.textContent).not.toContain('⚑');
    expect(a.textContent).toContain('A person can a.');
    expect(b.textContent).toContain('≈$9.00');
    expect(b.textContent).toContain('900k tok');
    expect(b.textContent).toContain('2 reviews');
    expect(b.textContent).toContain('keep high 2026-10-29');
    expect(a.textContent).toContain('never decided');
    expect(a.textContent).toContain('1 of 2 rules only here; also other (1)');
    expect(view.querySelector<HTMLAnchorElement>('.ccard[data-slug=a]')!.getAttribute('href')).toBe('#/o/a/capabilities/a');
    expect(view.querySelector<HTMLAnchorElement>('tr.cap[data-slug=a] a')!.getAttribute('href')).toBe('#/o/a/capabilities/a');
  });

  // req:web.views/capabilities-list@d74a43a5
  it('flag filter: ⚑ only lists the flagged ones, and the switch says which it shows', () => {
    const o = list();
    const all = renderCapabilities(capabilities(), o);
    all.querySelector<HTMLButtonElement>('button.flag-only')!.click();
    expect(o.onFilter).toHaveBeenCalledWith(true);
    const only = renderCapabilities(capabilities(), list({ flaggedOnly: true }));
    expect(slugs(only, 'tr.cap')).toEqual(['a']);
    expect(slugs(only, '.ccard')).toEqual(['a']);
    expect(only.querySelector('button.flag-only')!.getAttribute('aria-pressed')).toBe('true');
  });

  // req:web.views/capabilities-list@d74a43a5
  it('a report without capabilities says there are none, not that none is flagged', () => {
    const none = capabilities();
    none.report.capabilities = [];
    expect(renderCapabilities(none, list()).querySelector('.empty')!.textContent).toBe('No capabilities: CAPABILITIES.md holds none.');
    const c = capabilities();
    c.report.capabilities = c.report.capabilities.filter((x) => !x.flagged);
    expect(renderCapabilities(c, list({ flaggedOnly: true })).querySelector('.empty')!.textContent).toBe('No capability is flagged.');
  });

  // req:web.views/capabilities-list@d74a43a5
  it('figures and empties: dollars, tokens, live without, never decided, the notes, and the empty texts', () => {
    const c = capabilities();
    c.report.capabilities = [
      cost('b', 9, { spend: { dollars: 9, work_tokens: 900_000, cache_read_tokens: 0, active_seconds: 0 } }),
      cost('c', 12.34, { spend: { dollars: 12.34, work_tokens: 1_300_000, cache_read_tokens: 0, active_seconds: 0 }, overlap: { only: 2, shared: [] } }),
    ];
    c.report.unattributed = { dollars: 1, work_tokens: 0, cache_read_tokens: 0, active_seconds: 0 };
    c.report.architecture_only = { dollars: 2, work_tokens: 0, cache_read_tokens: 0, active_seconds: 0 };
    const view = renderCapabilities(c, list());
    const row = (slug: string) => view.querySelector<HTMLElement>(`tr.cap[data-slug=${slug}]`)!.textContent;
    for (const text of ['≈$9.00', '900k tok', '1 of 2 rules only here; also other (1)', 'never decided']) expect(row('b')).toContain(text);
    for (const text of ['≈$12.3', '1.3M tok', 'all its rules only here']) expect(row('c')).toContain(text);
    expect(row('c')).not.toContain('≈$12.34');
    expect(tokens(850)).toBe('850');
    expect(tokens(1_000)).toBe('1k');
    // The card holds the same facts, with a bar for its share of the costliest.
    const card = view.querySelector<HTMLElement>('.ccard[data-slug=b]')!;
    for (const text of ['≈$9.00', '900k tok', '1 of 2 rules only here; also other (1)', 'never decided']) expect(card.textContent).toContain(text);
    expect(card.querySelector<HTMLElement>('.cbar i')!.style.width).toBe('73%');
    expect(view.querySelector<HTMLElement>('.ccard[data-slug=c] .cbar i')!.style.width).toBe('100%');
    const note = [...view.querySelectorAll('.note')].map((n) => n.textContent).find((t) => t?.startsWith('Unattributed'))!;
    expect(note).toContain('≈$1.00');
    expect(note).toContain('≈$2.00');
    const none = capabilities();
    none.report.capabilities = [];
    expect(renderCapabilities(none, list()).querySelector('.empty')!.textContent).toBe('No capabilities: CAPABILITIES.md holds none.');
    expect(renderCapabilities(c, list({ flaggedOnly: true })).querySelector('.empty')!.textContent).toBe('No capability is flagged.');
  });
});

describe('capability page', () => {
  const withText = () => {
    const c = capabilities();
    const a = c.report.capabilities.find((x) => x.slug === 'a')!;
    a.text = '### Capability: a\nRun `clm land` safely.\n\n#### Scenario: s\n- GIVEN a\n- WHEN b\n- THEN c';
    return c;
  };

  // req:web.views/capability-page@1b546acc
  it('page sections in order: the entry in full as Markdown, and clm capabilities named for what the Worker lacks', () => {
    const view = renderCapability(withText(), 'a', page());
    const secs = [...view.querySelectorAll<HTMLElement>('section[data-sec]')].map((s) => s.dataset.sec);
    expect(secs).toEqual(['entry', 'cost', 'workarounds', 'without', 'rules', 'decisions']);
    expect(view.querySelector('h1')!.textContent).toContain('a ⚑');
    const entry = view.querySelector('section[data-sec=entry]')!;
    expect(entry.querySelector('code')!.textContent).toBe('clm land');
    expect(entry.textContent).toContain('Scenario: s');
    expect(entry.textContent).not.toContain('`');
    // The statement and the why are the entry's text, not drawn again.
    expect(view.querySelector('.statement')).toBeNull();
    expect(view.querySelector('section[data-sec=why]')).toBeNull();
    expect(view.querySelector('section[data-sec=cost]')!.textContent).toContain('clm capabilities a');
    expect(view.querySelector('section[data-sec=without]')!.textContent).toContain('1 of its 2 rules serve only it; the rest also serve other (1)');
    const rules = view.querySelector<HTMLDetailsElement>('section[data-sec=rules] details')!;
    expect(rules.open).toBe(false);
    expect(rules.textContent).toContain('n/a-one');
    expect(view.querySelector('section[data-sec=decisions]')!.textContent).toContain('never decided');
    expect(view.querySelector<HTMLAnchorElement>('a.back')!.getAttribute('href')).toBe('#/o/a/capabilities');
  });

  // req:web.views/capability-page@1b546acc
  it('workaround details show its text: summary its title, body its text', () => {
    const view = renderCapability(capabilities(), 'a', page());
    const d = view.querySelector<HTMLDetailsElement>('section[data-sec=workarounds] details')!;
    expect(d.querySelector('summary')!.textContent).toBe('Judge ignores pipes');
    expect(d.textContent).toContain('Builds pipe output.');
    expect(d.textContent).toContain('n/a-one');
  });

  // req:web.views/capability-page@1b546acc
  it('an unknown slug says so', () => {
    expect(renderCapability(capabilities(), 'zzz', page()).textContent).toContain('zzz');
    expect(renderCapability(capabilities(), 'nope', page()).querySelector('.empty')!.textContent).toBe('The report holds no capability nope.');
  });
});

describe('decide', () => {
  // req:web.views/decide-buttons@4075a6f6
  it('decide one request per tap: a second tap while the first is in flight sends nothing', () => {
    const o = page();
    const view = renderCapability(capabilities(), 'a', o);
    const names = decideButtons(view).map((b) => b.textContent);
    expect(names).toEqual(['Keep', 'Cut', 'Redesign', 'Accept as is']);
    const cut = decideButtons(view)[1];
    cut.click();
    cut.click();
    decideButtons(view)[0].click();
    expect(o.onDecide).toHaveBeenCalledTimes(1);
    expect(o.onDecide).toHaveBeenCalledWith('cut');
    expect(decideButtons(view).every((b) => b.disabled)).toBe(true);
  });

  // req:web.views/decide-buttons@4075a6f6
  it('busy: while a decision is in flight, a page drawn again keeps the buttons disabled', () => {
    expect(decideButtons(renderCapability(capabilities(), 'a', page({ busy: true }))).every((b) => b.disabled)).toBe(true);
  });

  // req:web.views/decide-buttons@4075a6f6
  it('choosing a value calls onValue, and the chosen one is pressed', () => {
    const o = page({ value: 'medium' });
    const view = renderCapability(capabilities(), 'a', o);
    const values = [...view.querySelectorAll<HTMLButtonElement>('button.value')];
    expect(values.map((v) => [v.textContent, v.getAttribute('aria-pressed')])).toEqual([
      ['high', 'false'],
      ['medium', 'true'],
      ['low', 'false'],
    ]);
    values[0].click();
    expect(o.onValue).toHaveBeenCalledWith('high');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('decide disabled offline, signed or without a value; the signed note gives the command', () => {
    for (const [c, o] of [
      [capabilities(), page({ offline: true })],
      [capabilities(), page({ value: null })],
      [capabilities({ signed: true }), page()],
    ] as const) {
      expect(decideButtons(renderCapability(c, 'a', o)).every((b) => b.disabled)).toBe(true);
    }
    expect(decideButtons(renderCapability(capabilities(), 'a', page())).every((b) => !b.disabled)).toBe(true);
    const signed = renderCapability(capabilities({ signed: true }), 'a', page());
    expect(signed.querySelector('.signed-note code')!.textContent).toBe('clm capabilities decide a --value high|medium|low --decision keep|cut|redesign|accept');
    expect(renderCapability(capabilities(), 'a', page()).querySelector('.signed-note')).toBeNull();
  });

  // req:web.views/decide-buttons@4075a6f6
  it('a note shows, with the command a 422 signed gave', () => {
    const view = renderCapability(capabilities(), 'a', page({ note: { text: 'Signed only.', command: 'clm capabilities decide a --value low --decision cut' } }));
    expect(view.querySelector('.decide-note')!.textContent).toContain('Signed only.');
    expect(view.querySelector('.decide-note code')!.textContent).toBe('clm capabilities decide a --value low --decision cut');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('a 422 signed note in a signed report shows one command, the Worker’s; without one, no dangling sentence', () => {
    const command = 'clm capabilities decide a --value low --decision cut';
    const view = renderCapability(capabilities({ signed: true }), 'a', page({ note: { text: 'Not decided: signed only.', command } }));
    expect([...view.querySelectorAll('code')].filter((c) => c.textContent!.startsWith('clm capabilities decide')).map((c) => c.textContent)).toEqual([command]);
    const bare = renderCapability(capabilities({ signed: true }), 'a', page({ note: { text: 'Not decided: signed only.' } }));
    expect(bare.querySelector('.decide-note')!.textContent).toBe('Not decided: signed only.');
    expect(bare.querySelector('.signed-note')).not.toBeNull();
  });

  const answer = (status: number, body?: unknown) => new Response(body === undefined ? '' : JSON.stringify(body), { status });

  // req:web.views/decide-buttons@4075a6f6
  it('posts the version shown, the value and the decision, then loads the report again', async () => {
    const post = vi.fn(async () => answer(200, {}));
    const reload = vi.fn(async () => true);
    const note = await decide('/api/repos/o/a', 'capabilities', 'a', 'a1000000', 'low', 'cut', { post, reload });
    expect(post).toHaveBeenCalledTimes(1);
    expect(post).toHaveBeenCalledWith('/api/repos/o/a/capabilities/a/decide', { version: 'a1000000', value: 'low', decision: 'cut' });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(note.text).toContain('Decided');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('decide_409_reloads: a 409 loads the report again and says the capability changed', async () => {
    const reload = vi.fn(async () => true);
    const note = await decide('/b', 'capabilities', 'a', 'a1000000', 'low', 'cut', { post: async () => answer(409), reload });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(note.text).toContain('changed since it was shown');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('a 422 signed loads the report again and gives the Worker’s command; another failure says the status', async () => {
    const reload = vi.fn(async () => true);
    const command = 'clm capabilities decide a --value low --decision cut';
    const signed = await decide('/b', 'capabilities', 'a', 'v', 'low', 'cut', { post: async () => answer(422, { error: 'signed', command }), reload });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(signed.command).toBe(command);
    const other = await decide('/b', 'capabilities', 'a', 'v', 'low', 'cut', { post: async () => answer(502), reload: vi.fn(async () => true) });
    expect(other.text).toContain('502');
    // Gone from the tip, or its specs don't load: the page is drawn as the tip is now.
    const gone = vi.fn(async () => true);
    const refused = await decide('/b', 'capabilities', 'a', 'v', 'low', 'cut', { post: async () => answer(422, { error: 'capability' }), reload: gone });
    expect(gone).toHaveBeenCalledTimes(1);
    expect(refused.text).toContain('422');
    const signedOut = vi.fn();
    const out = await decide('/b', 'capabilities', 'a', 'v', 'low', 'cut', {
      post: async () => {
        throw new SignedOut();
      },
      reload: vi.fn(async () => true),
      signedOut,
    });
    expect(signedOut).toHaveBeenCalledTimes(1);
    expect(out.text).toContain('Signed out');
    const lost = await decide('/b', 'capabilities', 'a', 'v', 'low', 'cut', { post: async () => answer(409), reload: async () => false });
    expect(lost.text).toContain('Reload');
  });
  // A caller as web's shell is: the page's tap posts through `decide`, which loads the report
  // again through `reload`, and the page is drawn again with the note it gives.
  const wired = (post: Deciding['post'], fresh: () => Capabilities | null, extra: Partial<Deciding> = {}) => {
    let data = capabilities();
    let note: Note | null = null;
    const reload = vi.fn(async () => {
      const next = fresh();
      if (next) data = next;
      return next !== null;
    });
    let pending: Promise<void> = Promise.resolve();
    const onDecide = vi.fn((d: DecisionKind) => {
      const shown = data.report.capabilities.find((c) => c.slug === 'a')!;
      pending = decide('/api/repos/o/a', 'capabilities', 'a', shown.version, 'low', d, { post, reload, ...extra }).then((n) => {
        note = n;
      });
    });
    const view = renderCapability(data, 'a', page({ onDecide }));
    return { view, reload, onDecide, done: () => pending, redraw: () => renderCapability(data, 'a', page({ note })) };
  };
  const decidedCut = () => {
    const c = capabilities();
    c.report.capabilities[1].decisions = [{ at: '2026-11-24T10:00:00Z', writer: 'alice/web', value: 'low', decision: 'cut' }];
    return c;
  };

  // req:web.views/decide-buttons@4075a6f6
  it('one request per tap: two quick taps on Cut post the version shown once, the report is loaded again and the page shows the decision', async () => {
    const post = vi.fn(async () => answer(200, {}));
    const w = wired(post, decidedCut);
    const cut = w.view.querySelector<HTMLButtonElement>('button.decide[data-decision=cut]')!;
    cut.click();
    cut.click();
    await w.done();
    expect(post.mock.calls).toEqual([['/api/repos/o/a/capabilities/a/decide', { version: capabilities().report.capabilities[1].version, value: 'low', decision: 'cut' }]]);
    expect(w.reload).toHaveBeenCalledTimes(1);
    const after = w.redraw();
    expect(after.querySelector('section[data-sec=decisions]')!.textContent).toContain('cut (value low) by alice/web');
    expect(after.querySelector('.decide-note')!.textContent).toBe('Decided: cut, value low.');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('moved: a 409 says the capability changed since it was shown and loads the report again', async () => {
    // The Worker's tip holds the capability at a new version, which the page then shows.
    const moved = () => {
      const c = capabilities();
      c.report.capabilities[1].version = 'b2000000';
      c.report.capabilities[1].text = '### Capability: a\nA person can a, now in CSV.';
      return c;
    };
    const w = wired(async () => answer(409), moved);
    w.view.querySelector<HTMLButtonElement>('button.decide[data-decision=keep]')!.click();
    await w.done();
    expect(w.reload).toHaveBeenCalledTimes(1);
    const after = w.redraw();
    expect(after.querySelector('.decide-note')!.textContent).toContain('The capability changed since it was shown');
    expect(after.querySelector('section[data-sec=entry]')!.textContent).toContain('now in CSV');
  });

  // req:web.views/decide-buttons@4075a6f6
  it('refused: a 422, a 502, a post not sent, a session ended and a report that fails to load again each say so', async () => {
    const tap = async (post: Deciding['post'], fresh: () => Capabilities | null, extra: Partial<Deciding> = {}) => {
      const w = wired(post, fresh, extra);
      w.view.querySelector<HTMLButtonElement>('button.decide[data-decision=cut]')!.click();
      await w.done();
      return { text: w.redraw().querySelector('.decide-note')!.textContent, reload: w.reload };
    };
    const refused = await tap(async () => answer(422, { error: 'capability' }), () => capabilities());
    expect(refused.text).toBe('Not decided (422): the Worker refused the capability.');
    expect(refused.reload).toHaveBeenCalledTimes(1);
    const failed = await tap(async () => answer(502), () => capabilities());
    expect(failed.text).toBe('Not decided (502).');
    expect(failed.reload).not.toHaveBeenCalled();
    const offline = await tap(() => Promise.reject(new TypeError('Failed to fetch')), () => capabilities());
    expect(offline.text).toBe('Offline: decide when back online.');
    expect(offline.reload).not.toHaveBeenCalled();
    const signedOut = vi.fn();
    const out = await tap(() => Promise.reject(new SignedOut()), () => capabilities(), { signedOut });
    expect(signedOut).toHaveBeenCalledTimes(1);
    expect(out.text).toBe('Signed out. Sign in again to decide.');
    const lost = await tap(async () => answer(409), () => null);
    expect(lost.text!.endsWith('Its report could not be loaded again: Reload.')).toBe(true);
  });

  // req:web.views/decide-buttons@4075a6f6
  it('in flight: a page drawn while a decision is in flight keeps its buttons disabled and low pressed, and reports a value tapped', () => {
    const o = page({ busy: true, value: 'low' });
    const view = renderCapability(capabilities(), 'a', o);
    expect(decideButtons(view).every((b) => b.disabled)).toBe(true);
    expect([...view.querySelectorAll<HTMLButtonElement>('button.value')].map((v) => [v.textContent, v.getAttribute('aria-pressed')])).toEqual([
      ['high', 'false'],
      ['medium', 'false'],
      ['low', 'true'],
    ]);
    [...view.querySelectorAll<HTMLButtonElement>('button.value')].find((v) => v.textContent === 'medium')!.click();
    expect(vi.mocked(o.onValue).mock.calls).toEqual([['medium']]);
    decideButtons(view)[0].click();
    expect(o.onDecide).not.toHaveBeenCalled();
    // A signed report's page after a 422 signed shows the Worker's command once, and no other.
    const command = 'clm capabilities decide a --value low --decision cut';
    const signed = renderCapability(capabilities({ signed: true }), 'a', page({ note: { text: 'Not decided: this repository counts only records signed with your SSH key.', command } }));
    expect([...signed.querySelectorAll('code')].filter((c) => c.textContent!.startsWith('clm capabilities decide')).map((c) => c.textContent)).toEqual([command]);
  });
});

describe('repo markdown in the capabilities views', () => {
  const hostile = () => {
    const c = capabilities();
    c.report.capabilities[0].statement = 'Run `clm land` <img src=x onerror=alert(1)>';
    c.report.capabilities[0].text = '### Capability: b\nRun `clm land` <img src=x onerror=alert(1)>';
    return c;
  };

  // req:web.views/repo-markdown@342a93bb
  it('a statement in the list and the page shows code, no backtick and no onerror', () => {
    for (const view of [renderCapabilities(hostile(), list()), renderCapability(hostile(), 'b', page())]) {
      const code = view.querySelector('code');
      expect(code?.textContent).toBe('clm land');
      expect(view.textContent).not.toContain('`');
      expect(view.innerHTML).not.toContain('onerror');
    }
  });
});
