// The spec graph: every node inside its parent's box, the root's children inside the root,
// with the `depends_on` edges, laid out by ELK (layered, hierarchical, edges from dependent
// to dependency). Each card shows the status, requirements by coverage state and owners
// the `view` node's `spec` model gives it, and, once the implementation state arrives, a
// ring in its queue state's colour (web/graph-view), and `⚠` with an amber outline on a node
// over its size budget (web/oversized-node).

import type { ElkExtendedEdge, ElkNode } from 'elkjs/lib/elk.bundled.js';
import { div, el, svg } from './dom';
import type { Implementation, Snapshot, SpecNode, SpecView } from './types';

const W = 150;
const H = 66;
const PAD = 40;
/** Room in a box for its header, and around its children. */
const BOX = { top: 44, side: 14, bottom: 14 };

export interface Point {
  x: number;
  y: number;
}

export interface Rect extends Point {
  width: number;
  height: number;
}

export interface Layout {
  /** Every drawn node's absolute place on the stage, boxes and cards alike. */
  nodes: Map<string, Rect>;
  edges: { from: string; to: string; points: Point[] }[];
  width: number;
  height: number;
}

/** The stage's pan and zoom, which a redraw keeps (web/live-refresh). */
export interface Viewport {
  x: number;
  y: number;
  scale: number;
}

/** Each node's children, by parent ID; `null` holds the roots. */
function tree(spec: SpecView): Map<string | null, SpecNode[]> {
  const children = new Map<string | null, SpecNode[]>();
  for (const n of spec.nodes) children.set(n.parent, [...(children.get(n.parent) ?? []), n]);
  return children;
}

/** The root, which frames the graph and isn't drawn. */
export function rootOf(spec: SpecView): SpecNode | undefined {
  return spec.nodes.find((n) => n.parent === null);
}

/** The nodes the graph draws: everything under the root. */
export function drawn(spec: SpecView): SpecNode[] {
  return spec.nodes.filter((n) => n.parent !== null);
}

export async function layoutGraph(spec: SpecView): Promise<Layout> {
  const children = tree(spec);
  const shown = drawn(spec);
  const ids = new Set(shown.map((n) => n.node));
  const elkNode = (n: SpecNode): ElkNode => {
    const kids = children.get(n.node) ?? [];
    if (kids.length === 0) return { id: n.node, width: W, height: H };
    return {
      id: n.node,
      layoutOptions: { 'elk.padding': `[top=${BOX.top},left=${BOX.side},bottom=${BOX.bottom},right=${BOX.side}]` },
      children: kids.map(elkNode),
    };
  };
  const edges: ElkExtendedEdge[] = shown.flatMap((n) =>
    n.depends_on.filter((d) => ids.has(d)).map((d) => ({ id: `${n.node}->${d}`, sources: [n.node], targets: [d] })),
  );
  // ELK is most of the app's code, so it loads only when a graph is first drawn.
  const { default: ELK } = await import('elkjs/lib/elk.bundled.js');
  const elk = new ELK();
  const root = rootOf(spec);
  const graph: ElkNode = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.spacing.nodeNode': '40',
      'elk.layered.spacing.nodeNodeBetweenLayers': '50',
      'elk.edgeRouting': 'SPLINES',
    },
    children: (root ? children.get(root.node) ?? [] : shown.filter((n) => !ids.has(n.parent ?? ''))).map(elkNode),
    edges,
  };
  const out = await elk.layout(graph);
  const layout: Layout = { nodes: new Map(), edges: [], width: (out.width ?? 0) + PAD, height: (out.height ?? 0) + PAD };
  // Children sit relative to their parent's corner; the stage wants absolute places.
  const place = (n: ElkNode, ox: number, oy: number) => {
    for (const c of n.children ?? []) {
      const x = ox + (c.x ?? 0);
      const y = oy + (c.y ?? 0);
      layout.nodes.set(c.id, { x, y, width: c.width ?? W, height: c.height ?? H });
      place(c, x, y);
    }
  };
  place(out, PAD / 2, PAD / 2);
  const offset = (container?: string): Point => {
    const at = container && container !== 'root' ? layout.nodes.get(container) : undefined;
    return at ? { x: at.x, y: at.y } : { x: PAD / 2, y: PAD / 2 };
  };
  for (const e of out.edges ?? []) {
    const s = e.sections?.[0];
    const points = s ? [s.startPoint, ...(s.bendPoints ?? []), s.endPoint] : [];
    const o = offset((e as ElkExtendedEdge & { container?: string }).container);
    layout.edges.push({ from: e.sources[0], to: e.targets[0], points: points.map((p) => ({ x: p.x + o.x, y: p.y + o.y })) });
  }
  return layout;
}

/** `n/m` requirements current, as the spec model counts them, or how far the marker index
 * has got while it's incomplete. */
export function proven(snap: Snapshot, n: SpecNode): string {
  if (snap.coverage.state === 'indexing') return `indexing ${Math.round(snap.coverage.done * 100)}%`;
  return `${n.current}/${n.total}`;
}

/** The colour of each queue state (`colour-meanings`); one the app doesn't know is
 * `OTHER`, which the legend names only while a ring uses it. */
const OTHER = 'var(--muted)';
const RING: Record<string, string> = {
  ready: 'var(--sky)',
  claimed: 'var(--amber)',
  blocked: 'var(--rose)',
};

/** One entry per colour the graph uses, naming its meanings on the graph. */
const LEGEND: [string, string][] = [
  ['var(--green)', 'active · current'],
  ['var(--sky)', 'unbuilt · ready'],
  ['var(--amber)', 'behind · claimed'],
  ['var(--rose)', 'obsolete · blocked'],
  ['var(--accent)', 'touched by PR'],
];

/** Traces the edges of `selected` (web.views/edge-direction): each edge from a component
 * that depends on it in the incoming colour, each edge to a component it depends on in the
 * outgoing colour, the others dimmed, and the legend naming the two colours; with none
 * selected, every edge alike. Clears what an earlier call drew, so selecting redraws nothing
 * and the graph keeps its pan and zoom. */
export function traceEdges(graph: HTMLElement, selected: string | null) {
  for (const p of graph.querySelectorAll<SVGPathElement>('path.e')) {
    p.classList.remove('ein', 'eout', 'edim');
    const kind = selected === null ? null : p.getAttribute('data-to') === selected ? 'in' : p.getAttribute('data-from') === selected ? 'out' : 'dim';
    if (kind) p.classList.add(`e${kind}`);
    p.setAttribute('marker-end', kind === 'in' || kind === 'out' ? `url(#arr-${kind})` : 'url(#arr)');
  }
  for (const k of ['in', 'out']) {
    const entry = graph.querySelector<HTMLElement>(`.legend .edge-${k}`);
    if (entry) entry.hidden = selected === null;
  }
}

/** The queue state of each node that isn't converged, as the `flight` model gives it; none
 * before the implementation state arrives or while its coverage is indexing. */
export function queueStates(impl: Implementation | null): Map<string, string> {
  if (!impl || impl.coverage.state !== 'complete') return new Map();
  return new Map(impl.flight.nodes.filter((n) => n.state !== 'converged').map((n) => [n.node, n.state]));
}

/** Rings each drawn node in its queue state's colour and names the state on it, clearing
 * what an earlier call drew, so the graph takes the states in place when they arrive. */
export function ringGraph(graph: HTMLElement, states: Map<string, string>) {
  let other = false;
  for (const n of graph.querySelectorAll<HTMLElement>('.n[data-node], .grp[data-node]')) {
    const state = states.get(n.dataset.node!);
    for (const c of [...n.classList]) if (c === 'ring' || c.startsWith('q-')) n.classList.remove(c);
    n.style.removeProperty('--ring');
    n.querySelector('.q')?.remove();
    const labelled = n.matches('.grp') ? n.querySelector<HTMLElement>('.gh')! : n;
    labelled.setAttribute('aria-label', `${labelled.dataset.label}${state ? `, ${state}` : ''}`);
    if (!state) continue;
    n.classList.add('ring', `q-${state}`);
    n.style.setProperty('--ring', RING[state] ?? OTHER);
    other ||= !(state in RING);
    const title = n.querySelector('.t')!;
    title.insertBefore(el('span', { class: 'q' }, state), title.lastElementChild);
  }
  const legend = graph.querySelector<HTMLElement>('.legend .other');
  if (legend) legend.hidden = !other;
}

function path(points: Point[]): string {
  if (points.length === 0) return '';
  return points.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(' ');
}

/** The card's label: the node's ID, after `⚠` when the `spec` model marks it oversized
 * (web/oversized-node). */
function cardName(n: SpecNode): string {
  return n.oversized === true ? `⚠ ${n.node}` : n.node;
}

/** The bar, segmented by coverage state, and the line under it. While the index is
 * incomplete the counts mean nothing, so the bar is empty. */
function face(snap: Snapshot, n: SpecNode): HTMLElement[] {
  const parts: [string, number, string][] = [
    ['cur', n.current, 'var(--green)'],
    ['beh', n.behind, 'var(--amber)'],
    ['spd', n.unbuilt, 'var(--sky)'],
  ];
  const segments =
    snap.coverage.state === 'complete' && n.total > 0
      ? parts.filter(([, count]) => count > 0).map(([cls, count, colour]) => el('i', { class: cls, style: `width:${(count / n.total) * 100}%;background:${colour}` }))
      : [];
  return [
    div('p', ...segments),
    div(
      'f',
      `${n.status} · ${proven(snap, n)}`,
      n.obsolete > 0 ? el('span', { class: 'obs', title: `${n.obsolete} obsolete` }, `✕${n.obsolete}`) : '',
      n.owners.length ? el('span', { class: 'own' }, ` ${n.owners.join(' ')}`) : '',
    ),
  ];
}

export function renderGraph(
  snap: Snapshot,
  touched: Set<string>,
  layout: Layout,
  onSelect: (id: string | null) => void,
  selected?: string,
  view?: Viewport,
  onView?: (v: Viewport) => void,
  states: Map<string, string> = new Map(),
): HTMLElement {
  const root = rootOf(snap.spec);
  const children = tree(snap.spec);
  const lines = svg(
    'svg',
    { width: layout.width, height: layout.height, viewBox: `0 0 ${layout.width} ${layout.height}` },
    svg(
      'defs',
      {},
      svg(
        'marker',
        { id: 'arr', viewBox: '0 0 10 10', refX: 8, refY: 5, markerWidth: 7, markerHeight: 7, orient: 'auto' },
        svg('path', { d: 'M0,1L9,5L0,9z', style: 'fill:var(--edge)' }),
      ),
      ...(['in', 'out'] as const).map((k) =>
        svg(
          'marker',
          { id: `arr-${k}`, viewBox: '0 0 10 10', refX: 8, refY: 5, markerWidth: 7, markerHeight: 7, orient: 'auto' },
          svg('path', { d: 'M0,1L9,5L0,9z', style: `fill:var(--edge-${k})` }),
        ),
      ),
    ),
    layout.edges.map((e) =>
      svg('path', {
        class: touched.has(e.from) || touched.has(e.to) ? 'e eh' : 'e',
        d: path(e.points),
        'data-from': e.from,
        'data-to': e.to,
        'marker-end': 'url(#arr)',
      }),
    ),
  );
  const classes = (n: SpecNode, kind: string) => {
    const cls = [kind, n.status === 'active' ? 'active' : 'unbuilt'];
    if (touched.has(n.node)) cls.push('hit');
    if (n.oversized === true) cls.push('oversized');
    if (n.node === selected) cls.push('sel');
    return cls.join(' ');
  };
  /** Selecting the selected component again clears the selection; the shell moves the `sel` class. */
  const pick = (id: string) => onSelect(canvas.querySelector<HTMLElement>('.sel')?.dataset.node === id ? null : id);
  const at = (id: string): Rect => layout.nodes.get(id) ?? { x: 0, y: 0, width: W, height: H };
  const label = (n: SpecNode) =>
    `${n.node}, ${n.status}, ${proven(snap, n)}${n.oversized === true ? ', oversized' : ''}${n.owners.length ? `, ${n.owners.join(' ')}` : ''}${touched.has(n.node) ? ', touched' : ''}`;
  // Boxes first, then cards, so a card is always on top of the box it sits in.
  const nodes = drawn(snap.spec);
  const boxes = nodes
    .filter((n) => (children.get(n.node) ?? []).length > 0)
    .map((n) => {
      const r = at(n.node);
      const head = el('div', { class: 'gh', role: 'button', tabindex: 0, title: n.title, 'aria-label': label(n), 'data-label': label(n) }, div('t', el('span', {}, cardName(n)), el('span', { class: 's' })), ...face(snap, n));
      head.addEventListener('click', () => pick(n.node));
      return el('div', { class: classes(n, 'grp'), 'data-node': n.node, style: `left:${r.x}px;top:${r.y}px;width:${r.width}px;height:${r.height}px` }, head);
    });
  const cards = nodes
    .filter((n) => (children.get(n.node) ?? []).length === 0)
    .map((n) => {
      const r = at(n.node);
      const card = el(
        'button',
        { type: 'button', class: classes(n, 'n'), 'data-node': n.node, style: `left:${r.x}px;top:${r.y}px;width:${r.width}px;height:${r.height}px`, title: n.title, 'aria-label': label(n), 'data-label': label(n) },
        div('t', el('span', {}, cardName(n)), el('span', { class: 's' })),
        ...face(snap, n),
      );
      card.addEventListener('click', () => pick(n.node));
      return card;
    });
  const stage = el('div', { class: 'stage', style: `width:${layout.width}px;height:${layout.height}px` }, lines, boxes, cards);
  const canvas = div(
    'canvas',
    div('root', `${root?.node ?? ''} · root`),
    stage,
    div(
      'legend',
      ...LEGEND.map(([colour, meaning]) =>
        el('span', {}, el('span', { class: 'dot', style: `background:${colour};margin:0` }), colour === 'var(--amber)' && nodes.some((n) => n.oversized === true) ? `${meaning} · oversized` : meaning),
      ),
      el('span', { class: 'edge-in', hidden: true }, el('span', { class: 'dot', style: 'background:var(--edge-in);margin:0' }), 'depends on the selected'),
      el('span', { class: 'edge-out', hidden: true }, el('span', { class: 'dot', style: 'background:var(--edge-out);margin:0' }), 'the selected depends on'),
      el('span', { class: 'other', hidden: true }, el('span', { class: 'dot', style: `background:${OTHER};margin:0` }), 'other queue state'),
      el('span', {}, '→ depends on'),
    ),
  );
  ringGraph(canvas, states);
  traceEdges(canvas, selected ?? null);
  panZoom(canvas, stage, view, onView);
  return canvas;
}

/** Drag to pan, wheel or pinch to zoom, and buttons for both. Starts from `view` when
 * given, else fitted to the canvas; reports every change to `onView`. */
function panZoom(canvas: HTMLElement, stage: HTMLElement, view?: Viewport, onView?: (v: Viewport) => void) {
  let scale = view?.scale ?? 1;
  let x = view?.x ?? 0;
  let y = view?.y ?? 0;
  const apply = () => {
    stage.style.transform = `translate(${x}px,${y}px) scale(${scale})`;
    onView?.({ x, y, scale });
  };
  /** Zoom by `f` (within the limits) keeping the stage point under the client point (cx, cy) fixed. */
  const zoom = (f: number, cx: number, cy: number) => {
    const r = canvas.getBoundingClientRect();
    const px = cx - r.left;
    const py = cy - r.top;
    const next = Math.min(3, Math.max(0.3, scale * f));
    if (next === scale) return;
    const k = next / scale;
    x = px - (px - x) * k;
    y = py - (py - y) * k;
    scale = next;
    apply();
  };
  const zoomCentre = (f: number) => {
    const r = canvas.getBoundingClientRect();
    zoom(f, r.left + r.width / 2, r.top + r.height / 2);
  };
  const fit = () => {
    const w = canvas.clientWidth;
    const sw = stage.offsetWidth;
    scale = w && sw ? Math.min(1, w / sw) : 1;
    x = w && sw ? Math.max(0, (w - sw * scale) / 2) : 0;
    y = 30;
    apply();
  };
  const pointers = new Map<number, Point>();
  let pinch = 0;
  canvas.addEventListener('pointerdown', (e) => {
    if ((e.target as Element).closest('.n, .gh, .zoom')) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    canvas.setPointerCapture?.(e.pointerId);
  });
  canvas.addEventListener('pointermove', (e) => {
    const last = pointers.get(e.pointerId);
    if (!last) return;
    if (pointers.size === 2) {
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      if (pinch) zoom(d / pinch, (a.x + b.x) / 2, (a.y + b.y) / 2);
      pinch = d;
      return;
    }
    x += e.clientX - last.x;
    y += e.clientY - last.y;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    apply();
  });
  const up = (e: PointerEvent) => {
    pointers.delete(e.pointerId);
    pinch = 0;
  };
  canvas.addEventListener('pointerup', up);
  canvas.addEventListener('pointercancel', up);
  canvas.addEventListener(
    'wheel',
    (e) => {
      e.preventDefault();
      zoom(e.deltaY < 0 ? 1.1 : 1 / 1.1, e.clientX, e.clientY);
    },
    { passive: false },
  );
  const btn = (label: string, text: string, f: () => void) => el('button', { type: 'button', 'aria-label': label, onclick: f as EventListener }, text);
  canvas.append(div('zoom', btn('Zoom in', '＋', () => zoomCentre(1.2)), btn('Zoom out', '－', () => zoomCentre(1 / 1.2)), btn('Fit', '⤢', fit)));
  if (view) apply();
  else requestAnimationFrame(fit);
}
