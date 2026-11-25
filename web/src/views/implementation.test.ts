import { describe, expect, it } from 'vitest';
import { renderActivity, renderImplementation } from './implementation';
import { ago, lapsesIn } from './time';
import { HOUR, NOW, blockers, flight, implementation, snapshot } from './fixtures';

const build = (impl = implementation(), snap = snapshot(), which: 'flight' | 'blockers' = 'flight') =>
  renderImplementation(impl, snap, { now: NOW, show: which, onShow: () => {} });
const entry = (e: HTMLElement, node: string) => e.querySelector<HTMLElement>(`.fn[data-node="${node}"]`)!;

describe('implementation view', () => {
  // req:web.views/implementation-view@d45af0b5
  it('claimed node: shows it claimed, its last push, and lapsing in 3 hours', () => {
    const b = entry(build(), 'b');
    expect(b.classList.contains('claimed')).toBe(true);
    expect(b.textContent).toContain('last push 1 h ago');
    expect(b.textContent).toContain('lapses in 3 h');
  });

  // req:web.views/implementation-view@d45af0b5
  it('draws the claim against the clock it is given, so times stay current between redraws', () => {
    const view = renderImplementation(implementation(), snapshot(), { now: NOW + HOUR, show: 'flight', onShow: () => {} });
    const b = entry(view, 'b');
    expect(b.textContent).toContain('last push 2 h ago');
    expect(b.textContent).toContain('lapses in 2 h');
  });

  // req:web.views/implementation-view@d45af0b5
  it('takes the last push from when the claim lapses, not from the rounded hours', () => {
    const f = flight();
    f.nodes[0] = { ...f.nodes[0], claim: { lapses_at: NOW + 3 * HOUR + 20 * 60, pushed_hours_ago: 1, lapses_in_hours: 3 } };
    const b = entry(build(implementation({ flight: f })), 'b');
    expect(b.textContent).toContain('last push 40 min ago');
    expect(b.textContent).toContain('lapses in 3 h 20 m');
  });

  // req:web.views/implementation-view@d45af0b5
  it('lists the pending nodes in the model’s order with their state and pending requirements by state', () => {
    const view = build();
    const nodes = [...view.querySelectorAll<HTMLElement>('.fn')].map((n) => n.dataset.node);
    expect(nodes).toEqual(['b', 'a']);
    const a = entry(view, 'a');
    expect(a.classList.contains('blocked')).toBe(true);
    expect(a.querySelector('.pend')?.textContent).toContain('1 behind');
    expect(a.textContent).toContain('a/x');
    expect(entry(view, 'b').querySelector('.pend')?.textContent).toContain('1 unbuilt');
  });

  // req:web.views/implementation-view@d45af0b5
  it('blocked node: names the backlog item and that it waits on you', () => {
    const a = entry(build(), 'a');
    expect(a.textContent).toContain('backlog/setup');
    expect(a.textContent).toContain('waits on you');
  });

  // req:web.views/implementation-view@d45af0b5
  it('blocked by a need: shows `a` blocked by `b/y`', () => {
    const f = flight();
    f.nodes[1] = {
      ...f.nodes[1],
      pending: [{ ...f.nodes[1].pending[0], blocked_by: [], waits_on: ['b/y'] }],
      blocked_by: [{ requirement: 'b/y', requirements: ['a/x'] }],
    };
    const a = entry(build(implementation({ flight: f })), 'a');
    expect(a.classList.contains('blocked')).toBe(true);
    expect(a.querySelector('.d')?.textContent).toBe('Blocked by b/y · a/x');
    expect(a.textContent).not.toContain('waits on you');
    expect(a.querySelector('.rq')?.textContent).toContain('blocked by b/y');
  });

  // req:web.views/implementation-view@d45af0b5
  it('a need links each component the queue names to its In flight entry', () => {
    let shown: [string, string?] | null = null;
    const f = flight();
    f.nodes[1] = { ...f.nodes[1], blocked_by: [{ requirement: 'b/y', components: ['c'], requirements: ['a/x'] }] };
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: (w, n) => (shown = [w, n]) });
    entry(view, 'a').querySelector<HTMLElement>('.d a')!.click();
    expect(shown).toEqual(['flight', 'c']);
  });

  // req:web.views/implementation-view@d45af0b5
  it('a blocker pending on another component than its ID names: names and links the queue’s components, and one without them is its ID alone', () => {
    const shown: string[] = [];
    const base = flight().nodes[1];
    const pend = (id: string) => [{ ...base.pending[0], id, blocked_by: [] }];
    const f = flight({
      nodes: [
        { ...base, node: 'b', pending: pend('b/z'), blocked_by: [{ requirement: 'b/gone', components: ['c', 'd'], requirements: ['b/z'] }] },
        { ...base, node: 'c', state: 'ready', pending: pend('c/w'), blocked_by: [] },
        { ...base, node: 'd', state: 'ready', pending: pend('d/w'), blocked_by: [] },
        { ...base, node: 'e', pending: pend('e/v'), blocked_by: [{ requirement: 'a/x', requirements: ['e/v'] }] },
      ],
    });
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: (_w, n) => n && shown.push(n) });
    const b = entry(view, 'b').querySelector<HTMLElement>('.d')!;
    expect(b.textContent).toContain('Blocked by b/gone on c, d');
    const links = [...b.querySelectorAll<HTMLAnchorElement>('a')];
    expect(links.map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['c', '#node-c'],
      ['d', '#node-d'],
    ]);
    for (const a of links) a.click();
    expect(shown).toEqual(['c', 'd']);
    const e = entry(view, 'e').querySelector<HTMLElement>('.d')!;
    expect(e.textContent).toContain('Blocked by a/x');
    expect(e.textContent).not.toContain(' on ');
    expect(e.querySelector('a')).toBeNull();
  });

  // req:web.views/implementation-view@d45af0b5
  it('blocked by an item and a need: shows both', () => {
    const f = flight();
    f.nodes[1] = {
      ...f.nodes[1],
      pending: [{ ...f.nodes[1].pending[0], waits_on: ['b/y'] }],
      blocked_by: [...f.nodes[1].blocked_by, { requirement: 'b/y', requirements: ['a/x'] }],
    };
    const a = entry(build(implementation({ flight: f })), 'a');
    expect(a.textContent).toContain('Blocked by backlog/setup');
    expect(a.textContent).toContain('Blocked by b/y');
    expect(a.querySelector('.rq')?.textContent).toContain('blocked by backlog/setup, b/y');
  });

  // req:web.views/implementation-view@d45af0b5
  it('waits on: a/x shows as blocked by b/y', () => {
    const f = flight();
    f.nodes[1] = { ...f.nodes[1], pending: [{ ...f.nodes[1].pending[0], blocked_by: [], waits_on: ['b/y'] }], blocked_by: [] };
    const a = entry(build(implementation({ flight: f })), 'a');
    expect(a.querySelector('.rq')?.textContent).toContain('blocked by b/y');
  });

  // req:web.views/implementation-view@d45af0b5
  it('no waiting field: a ready node shows as ready, naming nothing it waits on', () => {
    // Built in parts, so no source file names the field (web.views/queue-states).
    const field = `${'waiting'}_on`;
    for (const extra of [{}, { [field]: ['b'] }]) {
      const f = flight();
      f.nodes[1] = { ...f.nodes[1], state: 'ready', blocked_by: [], ...extra };
      const shown = entry(build(implementation({ flight: f })), 'a');
      expect(shown.classList.contains('ready')).toBe(true);
      expect(shown.querySelector('.top')?.textContent).toContain('Ready');
      expect(shown.textContent).not.toContain('waits on');
    }
  });

  // req:web.views/implementation-view@d45af0b5
  it('checked page: b links to page #7 and shows it as checked', () => {
    const impl = implementation({ pages: { 'impl/b': { number: 7, url: 'https://github.com/o/a/pull/7', checked: true } } });
    const b = entry(build(impl), 'b');
    const page = b.querySelector<HTMLAnchorElement>('a.page')!;
    expect(page.textContent).toBe('#7');
    expect(page.href).toBe('https://github.com/o/a/pull/7');
    expect(b.querySelector('.checked')?.textContent).toBe('checked');
  });

  // req:web.views/implementation-view@d45af0b5
  it('a page whose head was not checked says so, and one not yet known says nothing', () => {
    const not = entry(build(implementation({ pages: { 'impl/b': { number: 7, url: 'u', checked: false } } })), 'b');
    expect(not.querySelector('.checked')?.textContent).toBe('not checked');
    const unknown = entry(build(implementation({ pages: { 'impl/b': { number: 7, url: 'u' } } })), 'b');
    expect(unknown.querySelector('a.page')).not.toBeNull();
    expect(unknown.querySelector('.checked')).toBeNull();
    expect(entry(build(), 'b').querySelector('a.page')).toBeNull();
  });

  // req:web.views/implementation-view@d45af0b5
  it('a state it does not know shows by its name', () => {
    const f = flight();
    f.nodes[0] = { ...f.nodes[0], state: 'parked', claim: null };
    const b = entry(build(implementation({ flight: f })), 'b');
    expect(b.querySelector('.top .chip.mute:last-child')?.textContent).toBe('parked');
  });

  // req:web.views/implementation-view@d45af0b5
  it('change progress: shows 1 of 2 requirements current and not complete, in the model’s groups', () => {
    const view = build();
    const c = view.querySelector<HTMLElement>('.chg[data-change=c]')!;
    expect(c.closest<HTMLElement>('.grp-changes')?.dataset.status).toBe('applied');
    expect(c.textContent).toContain('1/2');
    expect(c.querySelector('.chip.ok')).toBeNull();
    const f = flight();
    f.changes[0].changes[0] = { ...f.changes[0].changes[0], current: 2, complete: true };
    const done = build(implementation({ flight: f })).querySelector<HTMLElement>('.chg[data-change=c]')!;
    expect(done.textContent).toContain('complete');
  });

  // req:web.views/implementation-view@d45af0b5
  it('shows the change groups in the model’s order', () => {
    const f = flight();
    f.changes = [
      { status: 'proposed', changes: [{ ...f.changes[0].changes[0], change: 'p' }] },
      ...f.changes,
    ];
    const groups = [...build(implementation({ flight: f })).querySelectorAll<HTMLElement>('.grp-changes')];
    expect(groups.map((g) => g.dataset.status)).toEqual(['proposed', 'applied']);
  });

  // req:web.views/implementation-view@d45af0b5
  it('counts pending nodes by state, live changes and requirements to do', () => {
    const view = build();
    const stat = (k: string) => view.querySelector<HTMLElement>(`.stat[data-stat=${k}] .v`)!.textContent;
    expect(stat('pending')).toBe('2');
    expect(view.querySelector<HTMLElement>('.stat[data-stat=pending]')!.textContent).toContain('1 claimed');
    expect(view.querySelector<HTMLElement>('.stat[data-stat=pending]')!.textContent).toContain('1 blocked');
    expect(stat('changes')).toBe('1');
    expect(stat('requirements')).toBe('2');
  });

  // req:web.views/implementation-view@d45af0b5
  it('says so when nothing is pending', () => {
    const f = flight({ nodes: [], totals: { pending_nodes: 0, by_state: {}, live_changes: 1, requirements_pending: 0 } });
    expect(build(implementation({ flight: f })).textContent).toContain('Nothing is pending');
  });

  // req:web.views/implementation-view@d45af0b5
  it('counts and empty: the counts as the totals give them, and the empty texts', () => {
    const f = flight({ totals: { pending_nodes: 2, by_state: { ready: 1, claimed: 1 }, live_changes: 3, requirements_pending: 5 } });
    const view = build(implementation({ flight: f }));
    const stat = (k: string) => view.querySelector<HTMLElement>(`.stat[data-stat=${k}]`)!;
    expect(stat('pending').querySelector('.v')!.textContent).toBe('2');
    expect(stat('pending').querySelector('.note')!.textContent).toBe('1 claimed, 1 ready');
    expect(stat('changes').querySelector('.v')!.textContent).toBe('3');
    expect(stat('requirements').querySelector('.v')!.textContent).toBe('5');
    const empty = build(implementation({ flight: flight({ nodes: [], changes: [], totals: { pending_nodes: 0, by_state: {}, live_changes: 0, requirements_pending: 0 } }) }));
    const notes = [...empty.querySelectorAll('.flight .note')].map((n) => n.textContent);
    expect(notes).toEqual(['Nothing is pending: every component is up to its spec.', 'No live changes.']);
  });
});

describe('blockers switch', () => {
  // req:web.views/blockers-view@3c687c71
  it('blocker weight: what waits on you first, then roots by weight, each with the nodes it holds up', () => {
    const b = blockers({
      roots: [
        { root: 'backlog/q', kind: 'backlog', state: 'question', waits_on_human: true, weight: 1, holds: [{ node: 'd', via: [] }] },
        { root: 'a', kind: 'node', state: 'claimed', waits_on_human: false, weight: 2, holds: [{ node: 'b', via: [] }, { node: 'c', via: [] }] },
      ],
    });
    const view = build(implementation({ blockers: b }), snapshot(), 'blockers');
    const roots = [...view.querySelectorAll<HTMLElement>('.rt')];
    expect(roots.map((r) => r.dataset.root)).toEqual(['backlog/q', 'a']);
    expect(roots[0].textContent).toContain('waits on you');
    expect(roots[0].querySelector('.top a')).toBeNull();
    expect(roots[0].querySelector('.top code')!.textContent).toBe('backlog/q');
    expect(roots[0].textContent).toContain('holds up 1');
    expect(roots[1].textContent).toContain('holds up 2');
    expect([...roots[1].querySelectorAll<HTMLElement>('.held')].map((h) => h.dataset.node)).toEqual(['b', 'c']);
    expect(view.querySelector('.fn')).toBeNull();
  });

  // req:web.views/blockers-view@3c687c71
  it('shows the chain to a node held up through others', () => {
    const b = blockers({ roots: [{ root: 'a', kind: 'node', state: 'ready', waits_on_human: false, weight: 2, holds: [{ node: 'b', via: [] }, { node: 'c', via: ['b'] }] }] });
    const view = build(implementation({ blockers: b }), snapshot(), 'blockers');
    expect(view.querySelector<HTMLElement>('.held[data-node=c]')?.textContent).toContain('via b');
  });

  // req:web.views/blockers-view@3c687c71
  it('chain: d reads d via b › c, the root links to its entry, and tapping d shows its In flight entry', () => {
    const shown: [string, string?][] = [];
    const b = blockers({ roots: [{ root: 'a', kind: 'node', state: 'claimed', waits_on_human: false, weight: 1, holds: [{ node: 'd', via: ['b', 'c'] }] }] });
    const view = renderImplementation(implementation({ blockers: b }), snapshot(), { now: NOW, show: 'blockers', onShow: (w, n) => shown.push([w, n]) });
    const d = view.querySelector<HTMLElement>('.held[data-node=d]')!;
    expect(d.textContent).toBe('d via b › c');
    const root = view.querySelector<HTMLAnchorElement>('.rt[data-root=a] .top a')!;
    expect(root.getAttribute('href')).toBe('#node-a');
    d.querySelector<HTMLElement>('a')!.click();
    expect(shown).toEqual([['flight', 'd']]);
  });

  // req:web.views/blockers-view@3c687c71
  it('every node links to its entry in In flight', () => {
    let shown: [string, string?] | null = null;
    const view = renderImplementation(implementation(), snapshot(), { now: NOW, show: 'blockers', onShow: (w, n) => (shown = [w, n]) });
    view.querySelector<HTMLElement>('.held[data-node=a] a')!.click();
    expect(shown).toEqual(['flight', 'a']);
    const sw = [...view.querySelectorAll<HTMLElement>('.switch button')];
    expect(sw.map((b) => b.textContent)).toEqual(['In flight', 'Blockers']);
    expect(sw[1].classList.contains('on')).toBe(true);
    sw[0].click();
    expect(shown).toEqual(['flight', undefined]);
  });

  // req:web.views/blockers-view@3c687c71
  it('nothing blocked: says so', () => {
    const view = build(implementation({ blockers: blockers({ roots: [] }) }), snapshot(), 'blockers');
    expect(view.textContent).toContain('Nothing is blocked');
  });
});

describe('times', () => {
  // req:web.views/implementation-view@d45af0b5
  it('says how long until a claim lapses', () => {
    expect(lapsesIn(NOW + 3 * HOUR, NOW)).toBe('3 h');
    expect(lapsesIn(NOW + 3 * HOUR + 22 * 60, NOW)).toBe('3 h 22 m');
    expect(lapsesIn(NOW + 12 * 60, NOW)).toBe('12 min');
    expect(lapsesIn(NOW - 1, NOW)).toBe('lapsed');
    expect(lapsesIn(NOW, NOW)).toBe('lapsed');
  });

  // req:web.views/implementation-view@d45af0b5
  it('times: claims drawn against the clock, a passed one reads lapsed, and a redraw an hour later moves them', () => {
    const claim = (node: string, lapses_at: number) => ({ ...flight().nodes[0], node, claim: { lapses_at, pushed_hours_ago: 0, lapses_in_hours: 0 } });
    const f = flight({ nodes: [claim('p', NOW + 3 * HOUR + 20 * 60), claim('q', NOW + 12 * 60), claim('r', NOW - 1)] });
    const draw = (now: number) => renderImplementation(implementation({ flight: f, expiry_hours: 4 }), snapshot(), { now, show: 'flight', onShow: () => {} });
    const line = (view: HTMLElement, node: string) => entry(view, node).querySelector('.d')!.textContent;
    const view = draw(NOW);
    expect(line(view, 'p')).toBe('Claimed on impl/p · last push 40 min ago · lapses in 3 h 20 m');
    expect(line(view, 'q')).toContain('lapses in 12 min');
    expect(line(view, 'r')).toMatch(/ · lapsed$/);
    expect(line(view, 'r')).not.toContain('lapses in');
    const later = draw(NOW + HOUR);
    expect(line(later, 'p')).toBe('Claimed on impl/p · last push 1 h ago · lapses in 2 h 20 m');
  });

  // req:web.views/implementation-view@d45af0b5
  it('says how long ago', () => {
    expect(ago(NOW - 38 * 60, NOW)).toBe('38 min ago');
    expect(ago(NOW - HOUR, NOW)).toBe('1 h ago');
    expect(ago(NOW - 3 * 24 * HOUR, NOW)).toBe('3 d ago');
    expect(ago(NOW - 10, NOW)).toBe('just now');
    expect(ago(NOW + 5 * 60, NOW)).toBe('just now');
  });
});

describe('activity feed', () => {
  // req:web.views/activity-feed@b94fa5e2
  it('approval shows: lists the push, then the approval of spec/x', () => {
    const feed = renderActivity(
      [
        { kind: 'approval', at: NOW - 300, actor: 'clunny', ref: 'spec/x' },
        { kind: 'push', at: NOW, actor: 'clunny', ref: 'impl/c' },
      ],
      NOW,
    );
    const evs = [...feed.querySelectorAll<HTMLElement>('.ev')];
    expect(evs.map((e) => e.dataset.kind)).toEqual(['push', 'approval']);
    expect(evs[0].querySelector('.tx')?.textContent).toBe('Pushed to impl/c');
    expect(evs[1].querySelector('.tx')?.textContent).toBe('clunny approved spec/x');
  });

  // req:web.views/activity-feed@b94fa5e2
  it('shows claims and lands, a land naming its branch', () => {
    const feed = renderActivity(
      [
        { kind: 'claim', at: NOW - 50, actor: 'x', ref: 'b' },
        { kind: 'land', at: NOW - 60, actor: 'bob', ref: 'spec/x' },
      ],
      NOW,
    );
    const evs = [...feed.querySelectorAll<HTMLElement>('.ev .tx')].map((e) => e.textContent);
    expect(evs).toEqual(['Claimed b', 'bob landed spec/x']);
  });

  // req:web.views/activity-feed@b94fa5e2
  it('what each reads: a claim, a land with its actor, another kind by its name, and an empty feed', () => {
    const feed = renderActivity(
      [
        { kind: 'claim', at: NOW - 120, actor: '', ref: 'impl/a' },
        { kind: 'land', at: NOW - 180, actor: 'alice', ref: 'spec/x' },
        { kind: 'merge', at: NOW - 240, actor: '', ref: 'main' },
      ],
      NOW,
    );
    const evs = [...feed.querySelectorAll<HTMLElement>('.ev')];
    expect(evs.map((e) => e.querySelector('.tx')!.textContent)).toEqual(['Claimed impl/a', 'alice landed spec/x', 'merge main']);
    expect(evs.map((e) => e.querySelector('.tm')!.textContent)).toEqual(['2 min ago', '3 min ago · alice', '4 min ago']);
    const empty = renderActivity([], NOW);
    expect(empty.querySelector('.ev')).toBeNull();
    expect(empty.querySelector('.note')?.textContent).toBe('No recent activity.');
  });
});
