// req:web/graph-view@7bdf4eb6
import { describe, expect, it } from 'vitest';
import { layoutGraph, queueStates, renderGraph, ringGraph } from './graph';
import { flight, implementation, nestedSnapshot, node, snapshot, spec } from '../test/fixtures';
import type { FlightNode, RequirementView } from './types';

const req = (id: string, state: string): RequirementView => ({ id, text: `${id}.`, version: '00000000', state, markers: state === 'unbuilt' ? [] : ['t.rs:1'] });

/** Nodes `a`, `b` and `c` under a root, `a` with 2 current requirements and 1 behind. */
function threeNodes() {
  const nodes = [
    node('root', { parent: null }),
    node('a', { requirements: [req('a/x', 'current'), req('a/y', 'current'), req('a/z', 'behind')] }),
    node('b', { requirements: [req('b/x', 'unbuilt')] }),
    node('c', { requirements: [req('c/x', 'current')] }),
  ];
  return snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
}

const pending = (n: string, state: string): FlightNode => ({ node: n, priority: 'P3', state, pending: [], claim: null, blocked_by: [] });

/** The implementation state with `a` claimed and `b` blocked, `c` converged. */
const queued = (nodes = [pending('a', 'claimed'), pending('b', 'blocked')]) => implementation({ flight: flight({ nodes }) });

const box = (e: Element) => {
  const s = (e as HTMLElement).style;
  return { x: parseFloat(s.left), y: parseFloat(s.top), w: parseFloat(s.width), h: parseFloat(s.height) };
};
const inside = (inner: ReturnType<typeof box>, outer: ReturnType<typeof box>) =>
  inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.w <= outer.x + outer.w && inner.y + inner.h <= outer.y + outer.h;

describe('graph view', () => {
  it('touched node: draws b -> a and marks only b', async () => {
    const snap = snapshot();
    const layout = await layoutGraph(snap.spec);
    const g = renderGraph(snap, new Set(['b']), layout, () => {});

    const cards = [...g.querySelectorAll<HTMLElement>('.n[data-node]')].map((n) => n.dataset.node);
    expect(cards.sort()).toEqual(['a', 'b']);
    expect(g.querySelector('.root')?.textContent).toContain('root');

    const edges = [...g.querySelectorAll('path.e')].map((p) => `${p.getAttribute('data-from')}->${p.getAttribute('data-to')}`);
    expect(edges).toEqual(['b->a']);

    const hit = [...g.querySelectorAll<HTMLElement>('.n.hit')].map((n) => n.dataset.node);
    expect(hit).toEqual(['b']);
    expect(g.querySelector('path.e')?.classList.contains('eh')).toBe(true);
  });

  it('shows each node status, requirements current and owners, as the spec model gives them', async () => {
    const snap = snapshot();
    snap.spec.nodes.find((n) => n.node === 'a')!.owners = ['@clunny'];
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const a = g.querySelector<HTMLElement>('.n[data-node=a]')!;
    const b = g.querySelector<HTMLElement>('.n[data-node=b]')!;
    expect(a.classList.contains('active')).toBe(true);
    expect(a.textContent).toContain('1/1');
    expect(a.textContent).toContain('@clunny');
    expect(b.classList.contains('unbuilt')).toBe(true);
    expect(b.textContent).toContain('0/2');
    expect(g.querySelector('.n.hit')).toBeNull();
  });

  it("nested node with owners: check.drift's card is inside check's box and shows @a", async () => {
    const snap = nestedSnapshot();
    const layout = await layoutGraph(snap.spec);
    const g = renderGraph(snap, new Set(), layout, () => {});
    const check = g.querySelector<HTMLElement>('.grp[data-node=check]')!;
    const drift = g.querySelector<HTMLElement>('.n[data-node="check.drift"]')!;
    expect(check).not.toBeNull();
    expect(drift.textContent).toContain('@a');
    expect(drift.textContent).toContain('unbuilt');
    expect(inside(box(drift), box(check))).toBe(true);
    expect(check.textContent).toContain('check');
    expect(check.textContent).toContain('1/1');
    // The edge from gate to check.drift crosses into the box.
    const edge = g.querySelector('path.e[data-from=gate][data-to="check.drift"]');
    expect(edge).not.toBeNull();
    // It runs from gate's card to check.drift's, inside the box, in the stage's coordinates.
    const points = edge!.getAttribute('d')!.split(/[ML]/).filter(Boolean).map((p) => p.trim().split(' ').map(Number));
    const near = (p: number[], r: ReturnType<typeof box>) => p[0] >= r.x - 2 && p[0] <= r.x + r.w + 2 && p[1] >= r.y - 2 && p[1] <= r.y + r.h + 2;
    const gate = box(g.querySelector('.n[data-node=gate]')!);
    expect(near(points[0], gate), `starts at gate ${points[0]} ${JSON.stringify(gate)}`).toBe(true);
    expect(near(points[points.length - 1], box(drift)), `ends at check.drift ${points[points.length - 1]} ${JSON.stringify(box(drift))}`).toBe(true);
    // The root frames everything and isn't a card.
    expect(g.querySelector('.n[data-node=clm]')).toBeNull();
    expect(g.querySelector('.root')?.textContent).toContain('clm');
  });

  it('says coverage is indexing instead of partial counts', async () => {
    const snap = snapshot({ coverage: { state: 'indexing', done: 0.4 } });
    for (const n of snap.nodes) for (const r of n.requirements) r.markers = null;
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    expect(g.querySelector('.n[data-node=a]')!.textContent).toContain('indexing 40%');
    expect(g.querySelector('.n[data-node=a]')!.textContent).not.toContain('1/1');
    // The model counts every requirement unbuilt while nothing is indexed: no segments.
    expect(g.querySelectorAll('.p i')).toHaveLength(0);
  });

  it('selecting a node reports it, a nested group too', async () => {
    const snap = nestedSnapshot();
    let picked = '';
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), (id) => (picked = id));
    g.querySelector<HTMLElement>('.n[data-node="check.drift"]')!.click();
    expect(picked).toBe('check.drift');
    g.querySelector<HTMLElement>('.grp[data-node=check] .gh')!.click();
    expect(picked).toBe('check');
  });

  it('keeps the pan and zoom it is given', async () => {
    const snap = snapshot();
    const view = { x: 40, y: 12, scale: 0.8 };
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, view);
    expect(g.querySelector<HTMLElement>('.stage')!.style.transform).toBe('translate(40px,12px) scale(0.8)');
  });

  it("a node behind its spec: a's card shows 2/3, a green and an amber segment and no sky one", async () => {
    const snap = threeNodes();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const a = g.querySelector<HTMLElement>('.n[data-node=a]')!;
    expect(a.querySelector('.f')!.textContent).toContain('2/3');
    const cur = a.querySelector<HTMLElement>('.p i.cur')!;
    const beh = a.querySelector<HTMLElement>('.p i.beh')!;
    expect(cur.getAttribute('style')).toContain('var(--green)');
    expect(parseFloat(cur.style.width)).toBeCloseTo(66.7, 1);
    expect(beh.getAttribute('style')).toContain('var(--amber)');
    expect(parseFloat(beh.style.width)).toBeCloseTo(33.3, 1);
    expect(a.querySelector('.p i.spd')).toBeNull();
    // A node never built has only the sky segment.
    const b = g.querySelector<HTMLElement>('.n[data-node=b]')!;
    expect([...b.querySelectorAll('.p i')].map((i) => i.className)).toEqual(['spd']);
    expect(b.querySelector<HTMLElement>('.p i.spd')!.getAttribute('style')).toContain('var(--sky)');
  });

  it('shows the obsolete count when there is one, and none otherwise', async () => {
    const snap = threeNodes();
    snap.spec.nodes.find((n) => n.node === 'a')!.obsolete = 2;
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    expect(g.querySelector('.n[data-node=a] .obs')!.textContent).toBe('✕2');
    expect(g.querySelector('.n[data-node=c] .obs')).toBeNull();
    expect(g.querySelector('.n[data-node=c]')!.textContent).not.toContain('✕');
  });

  it('a node with no requirements draws an empty bar and 0/0', async () => {
    const nodes = [node('root', { parent: null }), node('e')];
    const snap = snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const e = g.querySelector<HTMLElement>('.n[data-node=e]')!;
    expect(e.querySelectorAll('.p i')).toHaveLength(0);
    expect(e.textContent).toContain('0/0');
  });

  it("queue states: a's card carries the amber claimed ring, b's the rose blocked one, c's none", async () => {
    const snap = threeNodes();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, undefined, undefined, queueStates(queued()));
    const card = (id: string) => g.querySelector<HTMLElement>(`.n[data-node=${id}]`)!;
    expect(card('a').classList.contains('q-claimed')).toBe(true);
    expect(card('a').style.getPropertyValue('--ring')).toBe('var(--amber)');
    expect(card('a').querySelector('.q')!.textContent).toBe('claimed');
    expect(card('b').classList.contains('q-blocked')).toBe(true);
    expect(card('b').style.getPropertyValue('--ring')).toBe('var(--rose)');
    expect(card('b').querySelector('.q')!.textContent).toBe('blocked');
    expect(card('c').classList.contains('ring')).toBe(false);
    expect(card('c').querySelector('.q')).toBeNull();
    expect(card('c').style.getPropertyValue('--ring')).toBe('');
    expect(card('a').getAttribute('aria-label')).toContain('claimed');
  });

  it('a bar with a requirement in each state fills its track exactly', async () => {
    const nodes = [node('root', { parent: null }), node('t', { requirements: [req('t/x', 'current'), req('t/y', 'behind'), req('t/z', 'unbuilt')] })];
    const snap = snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const widths = [...g.querySelectorAll<HTMLElement>('.n[data-node=t] .p i')].map((i) => parseFloat(i.style.width));
    expect(widths).toHaveLength(3);
    expect(widths.reduce((a, b) => a + b, 0)).toBeCloseTo(100, 6);
  });

  it('queueStates passes every state through, one the app does not know too', () => {
    const states = queueStates(queued(['ready', 'claimed', 'waiting', 'blocked', 'parked'].map((s, i) => pending(`n${i}`, s))));
    expect([...states.values()]).toEqual(['ready', 'claimed', 'waiting', 'blocked', 'parked']);
  });

  it('rings colour by meaning: ready sky, claimed amber, blocked rose, others muted', async () => {
    const nodes = [node('root', { parent: null }), ...['r', 'cl', 'w', 'bl', 'x'].map((id) => node(id))];
    const snap = snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
    const states = queueStates(queued([pending('r', 'ready'), pending('cl', 'claimed'), pending('w', 'waiting'), pending('bl', 'blocked'), pending('x', 'parked')]));
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, undefined, undefined, states);
    const ring = (id: string) => g.querySelector<HTMLElement>(`.n[data-node=${id}]`)!.style.getPropertyValue('--ring');
    expect([ring('r'), ring('cl'), ring('w'), ring('bl'), ring('x')]).toEqual([
      'var(--sky)',
      'var(--amber)',
      'var(--muted)',
      'var(--rose)',
      'var(--muted)',
    ]);
    expect(g.querySelector('.n[data-node=x] .q')!.textContent).toBe('parked');
    // The muted ring is a colour the graph now uses, so the legend names it.
    const other = g.querySelector<HTMLElement>('.legend .other')!;
    expect(other.hidden).toBe(false);
    expect(other.textContent).toBe('other queue state');
    expect(other.querySelector<HTMLElement>('.dot')!.style.background).toBe('var(--muted)');
    ringGraph(g, queueStates(queued([pending('r', 'ready')])));
    expect(other.hidden).toBe(true);
  });

  it('no waiting state: the graph rings `waiting` as any state it does not know', async () => {
    // req:web/queue-states@340afeed
    const nodes = [node('root', { parent: null }), node('w'), node('x')];
    const snap = snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
    const states = queueStates(queued([pending('w', 'waiting'), pending('x', 'parked')]));
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, undefined, undefined, states);
    const ring = (id: string) => g.querySelector<HTMLElement>(`.n[data-node=${id}]`)!.style.getPropertyValue('--ring');
    expect(ring('w')).toBe('var(--muted)');
    expect(ring('w')).toBe(ring('x'));
  });

  it('gives no rings before the implementation state arrives, or while its coverage is indexing', () => {
    expect(queueStates(null).size).toBe(0);
    expect(queueStates(implementation({ coverage: { state: 'indexing', done: 0.5 }, flight: flight({ nodes: [pending('a', 'claimed')] }) })).size).toBe(0);
    // A node listed as converged has nothing pending: no ring.
    expect(queueStates(queued([pending('a', 'converged')])).size).toBe(0);
  });

  it('rings a drawn graph in place when the implementation state arrives, boxes too', async () => {
    const snap = nestedSnapshot();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const drift = g.querySelector<HTMLElement>('.n[data-node="check.drift"]')!;
    const check = g.querySelector<HTMLElement>('.grp[data-node=check]')!;
    expect(drift.classList.contains('ring')).toBe(false);
    ringGraph(g, queueStates(queued([pending('check.drift', 'ready'), pending('check', 'blocked')])));
    expect(g.querySelector('.n[data-node="check.drift"]')).toBe(drift);
    expect(drift.classList.contains('q-ready')).toBe(true);
    expect(drift.querySelector('.q')!.textContent).toBe('ready');
    expect(check.classList.contains('q-blocked')).toBe(true);
    expect(check.style.getPropertyValue('--ring')).toBe('var(--rose)');
    expect(check.querySelector('.gh .q')!.textContent).toBe('blocked');
    // A later state replaces the ring; a node no longer pending loses it.
    ringGraph(g, queueStates(queued([pending('check.drift', 'claimed')])));
    expect(drift.classList.contains('q-ready')).toBe(false);
    expect(drift.classList.contains('q-claimed')).toBe(true);
    expect(drift.querySelectorAll('.q')).toHaveLength(1);
    expect(check.classList.contains('ring')).toBe(false);
    expect(check.querySelector('.q')).toBeNull();
  });

  it('the legend names every colour the graph uses with its meanings', async () => {
    const snap = snapshot();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const entries = [...g.querySelectorAll<HTMLElement>('.legend > span')].filter((s) => !s.hidden).map((s) => [s.querySelector<HTMLElement>('.dot')?.style.background ?? '', s.textContent]);
    expect(entries).toEqual([
      ['var(--green)', 'active · current'],
      ['var(--sky)', 'unbuilt · ready'],
      ['var(--amber)', 'behind · claimed'],
      ['var(--rose)', 'obsolete · blocked'],
      ['var(--accent)', 'touched by PR'],
      ['', '→ depends on'],
    ]);
  });
});

// req:web/oversized-component@550a1fc1
describe('oversized nodes', () => {
  /** `a` oversized, `b` not, `c` with no size report. */
  const sized = () => {
    const snap = threeNodes();
    snap.spec.nodes = snap.spec.nodes.map((n) => (n.node === 'a' ? { ...n, oversized: true } : n.node === 'b' ? { ...n, oversized: false } : n));
    return snap;
  };
  const amber = (g: HTMLElement) => [...g.querySelectorAll<HTMLElement>('.legend > span')].find((s) => s.querySelector<HTMLElement>('.dot')?.style.background === 'var(--amber)')!.textContent;

  it("oversized and not: a's card shows ⚠ with the amber outline and the legend names oversized; b's and c's show neither", async () => {
    const snap = sized();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {});
    const card = (id: string) => g.querySelector<HTMLElement>(`.n[data-node=${id}]`)!;
    expect(card('a').querySelector('.t')!.textContent).toContain('⚠');
    expect(card('a').classList.contains('oversized')).toBe(true);
    expect(card('a').getAttribute('aria-label')).toContain('oversized');
    for (const id of ['b', 'c']) {
      expect(card(id).querySelector('.t')!.textContent).not.toContain('⚠');
      expect(card(id).classList.contains('oversized')).toBe(false);
    }
    expect(amber(g)).toBe('behind · claimed · oversized');
    const plain = threeNodes();
    expect(amber(renderGraph(plain, new Set(), await layoutGraph(plain.spec), () => {}))).toBe('behind · claimed');
  });

  it("queue ring wins the outline: a, oversized and blocked, shows ⚠ and the rose blocked ring", async () => {
    const snap = sized();
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, undefined, undefined, queueStates(queued([pending('a', 'blocked')])));
    const a = g.querySelector<HTMLElement>('.n[data-node=a]')!;
    expect(a.querySelector('.t')!.textContent).toContain('⚠');
    expect(a.classList.contains('q-blocked')).toBe(true);
    expect(a.style.getPropertyValue('--ring')).toBe('var(--rose)');
  });
});

// req:web/zoom-at-pointer@7f4f9eca
describe('zoom at the pointer', () => {
  /** A graph with a known view, whose canvas sits at (100, 50) and is 800 x 600. */
  async function zoomable() {
    const snap = threeNodes();
    let view = { x: 40, y: 30, scale: 1 };
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, view, (v) => (view = v));
    g.getBoundingClientRect = () => ({ left: 100, top: 50, width: 800, height: 600, right: 900, bottom: 650, x: 100, y: 50, toJSON: () => ({}) });
    Object.defineProperty(g, 'clientWidth', { value: 800 });
    Object.defineProperty(g, 'clientHeight', { value: 600 });
    return { g, view: () => view };
  }
  /** The stage point at a screen point, which zooming must leave where it is. */
  const under = (v: { x: number; y: number; scale: number }, sx: number, sy: number) => ({ x: (sx - 100 - v.x) / v.scale, y: (sy - 50 - v.y) / v.scale });

  it('wheel over a card keeps the point under the pointer', async () => {
    const { g, view } = await zoomable();
    const before = under(view(), 420, 300);
    g.dispatchEvent(new WheelEvent('wheel', { deltaY: -100, clientX: 420, clientY: 300, cancelable: true }));
    expect(view().scale).toBeGreaterThan(1);
    const after = under(view(), 420, 300);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1);
    expect(Math.abs(after.y - before.y)).toBeLessThan(1);
  });

  it('the buttons zoom around the canvas centre', async () => {
    const { g, view } = await zoomable();
    const before = under(view(), 500, 350);
    g.querySelector<HTMLElement>('button[aria-label="Zoom in"]')!.click();
    expect(view().scale).toBeCloseTo(1.2);
    const after = under(view(), 500, 350);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1);
    expect(Math.abs(after.y - before.y)).toBeLessThan(1);
    g.querySelector<HTMLElement>('button[aria-label="Zoom out"]')!.click();
    const out = under(view(), 500, 350);
    expect(view().scale).toBeCloseTo(1);
    expect(Math.abs(out.x - before.x)).toBeLessThan(1);
    expect(Math.abs(out.y - before.y)).toBeLessThan(1);
  });

  it('a pinch zooms around the midpoint of the pointers', async () => {
    const { g, view } = await zoomable();
    const ev = (type: string, id: number, x: number, y: number) => {
      const e = new MouseEvent(type, { clientX: x, clientY: y, bubbles: true }) as MouseEvent & { pointerId: number };
      Object.defineProperty(e, 'pointerId', { value: id });
      return e;
    };
    g.dispatchEvent(ev('pointerdown', 1, 300, 200));
    g.dispatchEvent(ev('pointerdown', 2, 500, 200));
    g.dispatchEvent(ev('pointermove', 2, 500, 200));
    g.dispatchEvent(ev('pointermove', 2, 600, 200));
    const mid = { x: 450, y: 200 };
    expect(view().scale).toBeGreaterThan(1);
    const before = under({ x: 40, y: 30, scale: 1 }, mid.x, mid.y);
    const after = under(view(), mid.x, mid.y);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1);
    expect(Math.abs(after.y - before.y)).toBeLessThan(1);
  });

  it('keeps the zoom limits', async () => {
    const { g, view } = await zoomable();
    for (let i = 0; i < 60; i++) g.dispatchEvent(new WheelEvent('wheel', { deltaY: -100, clientX: 420, clientY: 300, cancelable: true }));
    expect(view().scale).toBe(3);
    for (let i = 0; i < 200; i++) g.dispatchEvent(new WheelEvent('wheel', { deltaY: 100, clientX: 420, clientY: 300, cancelable: true }));
    expect(view().scale).toBe(0.3);
  });
});
