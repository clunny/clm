// The inspector a selected node opens
import { describe, expect, it, vi } from 'vitest';
import { renderInspector } from './inspector';
import { flight, implementation, review, snapshot } from './fixtures';

describe('node inspector', () => {
  // req:web.views/inspector@44a06d57
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

  // req:web.views/inspector@44a06d57
  it('names an unbuilt node unbuilt in its header', () => {
    const insp = renderInspector(snapshot(), null, [], 'b');
    expect(insp.querySelector('.top')?.textContent).toContain('unbuilt');
  });

  // req:web.views/inspector@44a06d57
  it('says the work in flight is still loading until the implementation state arrives', () => {
    const insp = renderInspector(snapshot(), null, [], 'b');
    expect(insp.querySelector('.work')?.textContent).toContain('Loading');
  });

  // req:web.views/inspector@44a06d57
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

  // req:web.views/inspector@44a06d57
  it('says when no spec branch touches the node', () => {
    const insp = renderInspector(snapshot(), null, [review()], 'a');
    expect(insp.querySelector('.inpr')?.textContent).toContain('No open PR changes it.');
  });
});

describe('work in flight', () => {
  const reqs = (insp: HTMLElement) => [...insp.querySelectorAll<HTMLElement>('.work .rq')].map((r) => r.textContent);

  // req:web.views/inspector-pending@bfc459c7
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

  // req:web.views/inspector-pending@bfc459c7
  it('nothing pending: b, not listed, says nothing is pending; c, listed with none, lists no requirement', () => {
    const f = flight();
    f.nodes = [{ ...f.nodes[0], node: 'c', pending: [] }];
    const impl = implementation({ flight: f });
    const b = renderInspector(snapshot(), impl, [], 'b');
    expect(reqs(b)).toEqual([]);
    expect(b.querySelector('.work')?.textContent).toContain('Nothing pending');
    expect(reqs(renderInspector(snapshot(), impl, [], 'c'))).toEqual([]);
  });

  // req:web.views/inspector-pending@bfc459c7
  it('indexing: says coverage is indexing and lists no requirement', () => {
    const impl = implementation({ coverage: { state: 'indexing', done: 0.4 } });
    const insp = renderInspector(snapshot(), impl, [], 'b');
    expect(insp.querySelector('.work')?.textContent).toContain('Coverage is indexing');
    expect(reqs(insp)).toEqual([]);
  });
});

describe('inspector overlay', () => {
  // req:web.views/inspector-overlay@628f0d43
  it('has a × button that closes it', () => {
    const onClose = vi.fn();
    const insp = renderInspector(snapshot(), null, [], 'b', onClose);
    const x = insp.querySelector<HTMLButtonElement>('button.close')!;
    expect(x.textContent).toBe('×');
    expect(x.getAttribute('aria-label')).toBe('Close');
    x.click();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // req:web.views/inspector-overlay@628f0d43
  it('closes on Escape while it is on the page, and not after it has left', () => {
    const onClose = vi.fn();
    const insp = renderInspector(snapshot(), null, [], 'b', onClose);
    document.body.append(insp);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    expect(onClose).not.toHaveBeenCalled();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(onClose).toHaveBeenCalledTimes(1);
    insp.remove();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // req:web.views/inspector-overlay@628f0d43
  it("its backlog items link to their pages: 'dark-mode' opens #/o/a/backlog/dark-mode", () => {
    const snap = snapshot();
    snap.backlog = [{ slug: 'dark-mode', kind: 'idea', title: 'Dark mode', nodes: ['a'], origin: null, blocks: [] }];
    const insp = renderInspector(snap, null, [], 'a', undefined, (slug) => `#/o/a/backlog/${slug}`);
    const link = insp.querySelector<HTMLAnchorElement>('.backlog a')!;
    expect(link.textContent).toContain('dark-mode');
    expect(link.getAttribute('href')).toBe('#/o/a/backlog/dark-mode');
  });

  // req:web.views/inspector-overlay@628f0d43
  it('closing: the × and Escape close the inspector on the page once each, and one taken off the page not at all; items without links are plain text', () => {
    const a = vi.fn();
    const other = vi.fn();
    const gone = renderInspector(snapshot(), null, [], 'b', other);
    document.body.append(gone);
    gone.remove();
    const insp = renderInspector(snapshot(), null, [], 'a', a);
    document.body.append(insp);
    insp.querySelector<HTMLButtonElement>('button.close')!.click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(a).toHaveBeenCalledTimes(2);
    expect(other).not.toHaveBeenCalled();
    insp.remove();
    expect(insp.querySelector('.backlog a')).toBeNull();
    expect(insp.querySelector('.backlog')!.textContent).toContain('crash');
  });
});

describe('what the inspector shows', () => {
  // req:web.views/inspector@44a06d57
  it('what it shows: n with its status, title, dependencies, owners, spec/x changes labelled as the review gives them, contract, changes and backlog; m says each section is empty', () => {
    const snap = snapshot();
    const n = { ...snap.nodes[1], id: 'n', title: 'N', status: 'unbuilt', depends_on: ['a'], contract: '**Calls**', requirements: [] };
    snap.nodes = [snap.nodes[0], snap.nodes[1], n];
    snap.spec.nodes = [
      ...snap.spec.nodes.filter((s) => s.node !== 'b'),
      { node: 'n', parent: 'root', title: 'N', status: 'unbuilt', owners: ['@x'], depends_on: ['a'], current: 1, behind: 0, unbuilt: 1, obsolete: 0, total: 2 },
      { node: 'm', parent: 'root', title: 'M', status: 'active', owners: [], depends_on: [], current: 0, behind: 0, unbuilt: 0, obsolete: 0, total: 0 },
    ];
    snap.changes = [
      { id: 'c1', status: 'applied', from: null, resolves: [], nodes: ['n'], current: 1, total: 2, complete: false },
      { id: 'c2', status: 'proposed', from: null, resolves: [], nodes: ['n'], current: null, total: 1, complete: false },
    ];
    snap.backlog = [{ slug: 'later', kind: 'idea', title: 'Later', nodes: ['n'], origin: null, blocks: [] }];
    const x = review({ branch: 'spec/x' });
    x.nodes = [
      {
        id: 'n',
        status: 'modified',
        frontmatter: null,
        contract: { before: 'Old.', after: 'New.' },
        design: { before: null, after: 'A design.' },
        requirements: { added: [{ id: 'n/r', before: null, after: 'It SHALL r.' }], modified: [], removed: [] },
        assets: { added: [], modified: [], removed: [] },
      },
    ];
    const y = review({ branch: 'spec/y' });
    y.nodes = [{ ...x.nodes[0], id: 'other' }];
    const insp = renderInspector(snap, null, [x, y], 'n');
    expect(insp.querySelector('.top')!.textContent).toContain('unbuilt · 1/2');
    expect(insp.textContent).toContain('N');
    expect(insp.querySelector('.deps')!.textContent).toBe('a');
    expect(insp.textContent).toContain('Owned by @x');
    const prs = insp.querySelector('.inpr')!;
    expect(prs.textContent).toContain('spec/x');
    expect(prs.textContent).not.toContain('spec/y');
    expect([...prs.querySelectorAll('.mini b')].map((b) => b.textContent)).toEqual(['contractmodified', 'designadded', 'n/radded']);
    expect(insp.querySelector('.contract strong')!.textContent).toBe('Calls');
    expect(insp.querySelector('.changes [data-change=c1]')!.textContent).toContain('1/2 requirements current');
    expect(insp.querySelector('.changes [data-change=c2]')!.textContent).toContain('coverage indexing');
    expect(insp.querySelector('.work .note')!.textContent).toBe('Loading work in flight…');
    expect(insp.querySelector('.backlog')!.textContent).toContain('later');

    const m = renderInspector(snap, implementation(), [x, y], 'm');
    expect(m.querySelector('.inpr')!.textContent).toContain('No open PR changes it.');
    expect(m.querySelector('.contract')!.textContent).toContain('Not in this snapshot.');
    expect(m.querySelector('.changes')!.textContent).toContain('No change touches it.');
    expect(m.querySelector('.backlog')!.textContent).toContain('None.');
  });
});
