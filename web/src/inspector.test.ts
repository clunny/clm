// req:web/graph-view@7bdf4eb6 (the inspector a selected node opens)
import { describe, expect, it } from 'vitest';
import { renderInspector } from './inspector';
import { flight, implementation, review, snapshot } from '../test/fixtures';

describe('node inspector', () => {
  it('shows the contract, dependencies, changes, work in flight and backlog of a node', () => {
    const insp = renderInspector(snapshot(), implementation(), [], 'b');
    expect(insp.querySelector('.contract')?.textContent).toContain('The b contract.');
    expect(insp.querySelector('.deps')?.textContent).toBe('a');
    expect([...insp.querySelectorAll('.changes .mini')].map((m) => m.getAttribute('data-change'))).toEqual(['c']);
    expect(insp.querySelector('.changes')?.textContent).toContain('1/2 requirements current');
    expect(insp.querySelector('.work')?.textContent).toContain('claimed');
    expect(insp.querySelector('.work')?.textContent).toContain('1 pending');
    expect(insp.querySelector('.backlog')?.textContent).toContain('later');
    expect(insp.querySelector('.backlog')?.textContent).not.toContain('crash');
  });

  it('names an unbuilt node unbuilt in its header', () => {
    const insp = renderInspector(snapshot(), null, [], 'b');
    expect(insp.querySelector('.top')?.textContent).toContain('unbuilt');
  });

  it('says the work in flight is still loading until the implementation state arrives', () => {
    const insp = renderInspector(snapshot(), null, [], 'b');
    expect(insp.querySelector('.work')?.textContent).toContain('Loading');
  });

  it("lists each spec branch's changes to the node, and only branches that touch it", () => {
    const touching = review();
    const other = review({ branch: 'spec/elsewhere' });
    other.nodes = [{ ...other.nodes[0], id: 'zzz' }];
    const insp = renderInspector(snapshot(), null, [touching, other], 'n');
    const prs = insp.querySelector('.inpr')!;
    expect(prs.textContent).toContain('spec/c');
    expect(prs.textContent).toContain('n/r');
    expect(prs.textContent).not.toContain('spec/elsewhere');
  });

  it('says when no spec branch touches the node', () => {
    const insp = renderInspector(snapshot(), null, [review()], 'a');
    expect(insp.querySelector('.inpr')?.textContent).toContain('No open PR changes it.');
  });
});

// req:web/inspector-pending@bfc459c7
describe('work in flight', () => {
  const reqs = (insp: HTMLElement) => [...insp.querySelectorAll<HTMLElement>('.work .rq')].map((r) => r.textContent);

  it('obsolete marker of a removed node: a shows ready and 2 pending, a/x behind and gone/y obsolete with its markers', () => {
    const f = flight({
      nodes: [
        {
          node: 'a',
          priority: 'P1',
          state: 'ready',
          pending: [
            { id: 'a/x', version: 'aaaa0000', state: 'behind', cited: ['aaaa9999'], markers: ['t.rs:1'], priority: 'P1', blocked_by: [], waits_on: [] },
            { id: 'gone/y', version: null, state: 'obsolete', cited: ['11112222'], markers: ['src/a.rs:12', 'tests/a.rs:3'], priority: 'P1', blocked_by: [], waits_on: [] },
          ],
          claim: null,
          blocked_by: [],
        },
      ],
    });
    const insp = renderInspector(snapshot(), implementation({ flight: f }), [], 'a');
    const work = insp.querySelector<HTMLElement>('.work')!;
    expect(work.querySelector('.chip')?.textContent).toBe('ready');
    expect(work.textContent).toContain('2 pending');
    expect(reqs(insp)).toEqual(['a/x behind', 'gone/y obsolete · src/a.rs:12, tests/a.rs:3']);
  });

  it('nothing pending: b, not listed, says nothing is pending; c, listed with none, lists no requirement', () => {
    const f = flight();
    f.nodes = [{ ...f.nodes[0], node: 'c', pending: [] }];
    const impl = implementation({ flight: f });
    const b = renderInspector(snapshot(), impl, [], 'b');
    expect(reqs(b)).toEqual([]);
    expect(b.querySelector('.work')?.textContent).toContain('Nothing pending');
    expect(reqs(renderInspector(snapshot(), impl, [], 'c'))).toEqual([]);
  });

  it('indexing: says coverage is indexing and lists no requirement', () => {
    const impl = implementation({ coverage: { state: 'indexing', done: 0.4 } });
    const insp = renderInspector(snapshot(), impl, [], 'b');
    expect(insp.querySelector('.work')?.textContent).toContain('Coverage is indexing');
    expect(reqs(insp)).toEqual([]);
  });
});
