import { describe, expect, it, vi } from 'vitest';
import { decide, renderCapabilities, renderCapability } from './capabilities';
import type { ListOptions, PageOptions } from './capabilities';
import { capabilities } from './fixtures';
import { SignedOut } from './api';

const list = (o: Partial<ListOptions> = {}): ListOptions => ({ flaggedOnly: false, onFilter: vi.fn(), link: (s) => `#/o/a/capabilities/${s}`, ...o });
const page = (o: Partial<PageOptions> = {}): PageOptions => ({ offline: false, busy: false, value: 'low', onValue: vi.fn(), onDecide: vi.fn(), back: '#/o/a/capabilities', note: null, ...o });
const slugs = (e: HTMLElement, sel: string) => [...e.querySelectorAll<HTMLElement>(sel)].map((r) => r.dataset.slug);
const decideButtons = (e: HTMLElement) => [...e.querySelectorAll<HTMLButtonElement>('button.decide')];

// req:web.views/capabilities-list@fde4e68b
describe('capabilities list', () => {
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

  it('a report without capabilities says there are none, not that none is flagged', () => {
    const none = capabilities();
    none.report.capabilities = [];
    expect(renderCapabilities(none, list()).querySelector('.empty')!.textContent).toBe('No capabilities: CAPABILITIES.md holds none.');
    const c = capabilities();
    c.report.capabilities = c.report.capabilities.filter((x) => !x.flagged);
    expect(renderCapabilities(c, list({ flaggedOnly: true })).querySelector('.empty')!.textContent).toBe('No capability is flagged.');
  });
});

// req:web/capability-page@853bfb39
describe('capability page', () => {
  it('page sections in order, and clm capabilities named for what the Worker lacks', () => {
    const view = renderCapability(capabilities(), 'a', page());
    const secs = [...view.querySelectorAll<HTMLElement>('section[data-sec]')].map((s) => s.dataset.sec);
    expect(secs).toEqual(['why', 'cost', 'workarounds', 'without', 'rules', 'decisions']);
    expect(view.querySelector('h1')!.textContent).toContain('a ⚑');
    const statement = view.querySelector('.statement')!;
    expect(statement.textContent).toBe('A person can a.');
    expect(statement.compareDocumentPosition(view.querySelector('section[data-sec=why]')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(view.querySelector('section[data-sec=why]')!.textContent).toContain('Without a, nothing works.');
    expect(view.querySelector('section[data-sec=cost]')!.textContent).toContain('clm capabilities a');
    expect(view.querySelector('section[data-sec=without]')!.textContent).toContain('1 of its 2 rules serve only it; the rest also serve other (1)');
    const rules = view.querySelector<HTMLDetailsElement>('section[data-sec=rules] details')!;
    expect(rules.open).toBe(false);
    expect(rules.textContent).toContain('n/a-one');
    expect(view.querySelector('section[data-sec=decisions]')!.textContent).toContain('never decided');
    expect(view.querySelector<HTMLAnchorElement>('a.back')!.getAttribute('href')).toBe('#/o/a/capabilities');
  });

  it('workaround details show its text: summary its title, body its text', () => {
    const view = renderCapability(capabilities(), 'a', page());
    const d = view.querySelector<HTMLDetailsElement>('section[data-sec=workarounds] details')!;
    expect(d.querySelector('summary')!.textContent).toBe('Judge ignores pipes');
    expect(d.textContent).toContain('Builds pipe output.');
    expect(d.textContent).toContain('n/a-one');
  });

  it('an unknown slug says so', () => {
    expect(renderCapability(capabilities(), 'zzz', page()).textContent).toContain('zzz');
  });
});

// req:web.views/decide-buttons@c1039d9e
describe('decide', () => {
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

  it('busy: while a decision is in flight, a page drawn again keeps the buttons disabled', () => {
    expect(decideButtons(renderCapability(capabilities(), 'a', page({ busy: true }))).every((b) => b.disabled)).toBe(true);
  });

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

  it('a note shows, with the command a 422 signed gave', () => {
    const view = renderCapability(capabilities(), 'a', page({ note: { text: 'Signed only.', command: 'clm capabilities decide a --value low --decision cut' } }));
    expect(view.querySelector('.decide-note')!.textContent).toContain('Signed only.');
    expect(view.querySelector('.decide-note code')!.textContent).toBe('clm capabilities decide a --value low --decision cut');
  });

  it('a 422 signed note in a signed report shows one command, the Worker’s; without one, no dangling sentence', () => {
    const command = 'clm capabilities decide a --value low --decision cut';
    const view = renderCapability(capabilities({ signed: true }), 'a', page({ note: { text: 'Not decided: signed only.', command } }));
    expect([...view.querySelectorAll('code')].filter((c) => c.textContent!.startsWith('clm capabilities decide')).map((c) => c.textContent)).toEqual([command]);
    const bare = renderCapability(capabilities({ signed: true }), 'a', page({ note: { text: 'Not decided: signed only.' } }));
    expect(bare.querySelector('.decide-note')!.textContent).toBe('Not decided: signed only.');
    expect(bare.querySelector('.signed-note')).not.toBeNull();
  });

  const answer = (status: number, body?: unknown) => new Response(body === undefined ? '' : JSON.stringify(body), { status });

  it('posts the version shown, the value and the decision, then loads the report again', async () => {
    const post = vi.fn(async () => answer(200, {}));
    const reload = vi.fn(async () => true);
    const note = await decide('/api/repos/o/a', 'capabilities', 'a', 'a1000000', 'low', 'cut', { post, reload });
    expect(post).toHaveBeenCalledTimes(1);
    expect(post).toHaveBeenCalledWith('/api/repos/o/a/capabilities/a/decide', { version: 'a1000000', value: 'low', decision: 'cut' });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(note.text).toContain('Decided');
  });

  it('decide_409_reloads: a 409 loads the report again and says the capability changed', async () => {
    const reload = vi.fn(async () => true);
    const note = await decide('/b', 'capabilities', 'a', 'a1000000', 'low', 'cut', { post: async () => answer(409), reload });
    expect(reload).toHaveBeenCalledTimes(1);
    expect(note.text).toContain('changed since it was shown');
  });

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
});
