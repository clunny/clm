import { describe, expect, it } from 'vitest';
import { layoutGraph, queueStates, renderGraph } from './graph';
import { renderImplementation } from './implementation';
import { NOW, flight, implementation, node, snapshot, spec } from './fixtures';
import type { FlightNode } from './types';

const pending = (n: string, state: string): FlightNode => ({ node: n, priority: 'P3', state, pending: [], claim: null, blocked_by: [] });

describe('queue states', () => {
  // req:web.views/queue-states@06e60952
  it('the totals give the states in the order claimed, ready, blocked, and know no waiting state', () => {
    const totals = (by_state: Record<string, number>) => {
      const f = flight({ totals: { pending_nodes: 4, by_state, live_changes: 1, requirements_pending: 4 } });
      const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
      return view.querySelector('.stat[data-stat=pending] .note')?.textContent;
    };
    expect(totals({ blocked: 1, ready: 1, claimed: 1 })).toBe('1 claimed, 1 ready, 1 blocked');
    // A state the app doesn't know comes after the known ones.
    expect(totals({ waiting: 1, blocked: 1 })).toBe('1 blocked, 1 waiting');
  });

  // req:web.views/queue-states@06e60952
  it('a waiting node has no label of its own: it shows its state as the app gets it', () => {
    const f = flight();
    f.nodes[1] = { ...f.nodes[1], state: 'waiting' };
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    const top = view.querySelector('#node-a .top')?.textContent ?? '';
    expect(top).toContain('waiting');
    expect(top).not.toContain('Waiting');
  });

  // req:web.views/queue-states@06e60952
  it('a node is drawn without reading paths from the flight model', () => {
    const f = flight();
    f.nodes[0] = { ...f.nodes[0], paths: ['web/.npmrc'] } as (typeof f.nodes)[0];
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    expect(view.textContent).not.toContain('web/.npmrc');
  });

  // req:web.views/queue-states@06e60952
  it('no interactive or waiting state: the totals, w by its own name, and w ringed as a state the app does not know', async () => {
    const f = flight({
      nodes: [pending('b', 'blocked'), pending('r', 'ready'), pending('c', 'claimed'), pending('w', 'waiting')],
      totals: { pending_nodes: 4, by_state: { blocked: 1, ready: 1, claimed: 1, waiting: 1 }, live_changes: 0, requirements_pending: 0 },
    });
    const impl = implementation({ flight: f });
    const view = renderImplementation(impl, snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    expect(view.querySelector('.stat[data-stat=pending] .note')?.textContent).toBe('1 claimed, 1 ready, 1 blocked, 1 waiting');
    const chips = [...view.querySelectorAll('#node-w .top .chip')].map((c) => c.textContent);
    expect(chips).toEqual(['P3', 'waiting']);
    expect([...view.querySelectorAll('#node-r .top .chip')].map((c) => c.textContent)).toEqual(['P3', 'Ready']);

    const nodes = [node('root', { parent: null }), node('w'), node('x')];
    const snap = snapshot({ nodes, spec: spec(nodes), changes: [], backlog: [] });
    const withPaths = { ...pending('x', 'parked'), paths: ['web/.npmrc'] } as FlightNode;
    const states = queueStates(implementation({ flight: flight({ nodes: [pending('w', 'waiting'), withPaths] }) }));
    const g = renderGraph(snap, new Set(), await layoutGraph(snap.spec), () => {}, undefined, undefined, undefined, states);
    const ring = (id: string) => g.querySelector<HTMLElement>(`.n[data-node=${id}]`)!.style.getPropertyValue('--ring');
    expect(ring('w')).toBe('var(--muted)');
    expect(ring('w')).toBe(ring('x'));
    // The graph reads no component's paths from the flight model either.
    expect(g.textContent).not.toContain('web/.npmrc');
  });
});
