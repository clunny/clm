// req:web/backlog-view@3bb10ff3
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Proposing, renderBacklog, waiting } from './backlog';
import { drafts } from './forms';
import { snapshot } from './fixtures';
import type { Proposal, Taken } from './types';

const slugs = (e: HTMLElement) => [...e.querySelectorAll<HTMLElement>('.bi')].map((b) => b.dataset.slug);

describe('backlog view', () => {
  it('filter: only items on the chosen node are shown', () => {
    const view = renderBacklog(snapshot(), null, 'a', () => {});
    expect(slugs(view).sort()).toEqual(['crash', 'setup']);
    const all = renderBacklog(snapshot(), null, null, () => {});
    expect(slugs(all).sort()).toEqual(['crash', 'later', 'setup']);
  });

  it('waiting on you: human-task and question items come first, naming the requirements they block', () => {
    const view = renderBacklog(snapshot(), null, null, () => {});
    const row = view.querySelector<HTMLElement>('.waiting')!;
    expect(row.textContent).toContain('Waiting on you');
    const setup = row.querySelector<HTMLElement>('.bi[data-slug=setup]')!;
    expect(setup.querySelector('.blocks')?.textContent).toBe('Blocks c/a');
    const order = slugs(view);
    expect(order.indexOf('setup')).toBeLessThan(order.indexOf('later'));
    expect(waiting(snapshot()).map((w) => [w.item.slug, w.blocks])).toEqual([['setup', ['c/a']]]);
  });

  it('a question blocking nothing yet still waits on you', () => {
    const snap = snapshot();
    snap.backlog.push({ slug: 'which', kind: 'question', title: 'Which?', nodes: ['b'], origin: null, blocks: [] });
    const view = renderBacklog(snap, null, null, () => {});
    const row = view.querySelector<HTMLElement>('.waiting')!;
    expect(slugs(row)).toEqual(['setup', 'which']);
    expect(row.querySelector<HTMLElement>('.bi[data-slug=which] .blocks')).toBeNull();
  });

  it('groups the rest by kind, each card with title, nodes and slug', () => {
    const view = renderBacklog(snapshot(), null, null, () => {});
    const bugs = view.querySelector<HTMLElement>('.col[data-kind=bug]')!;
    const card = bugs.querySelector<HTMLElement>('.bi[data-slug=crash]')!;
    expect(card.querySelector('.ttl')?.textContent).toBe('It crashes');
    expect(card.querySelector('.pill')?.textContent).toBe('a');
    expect(card.querySelector('.slug')?.textContent).toBe('crash');
    expect(view.querySelector('.col[data-kind=idea] .bi[data-slug=later]')).not.toBeNull();
  });

  it('picking a node pill filters', () => {
    let picked: string | null = 'x';
    const view = renderBacklog(snapshot(), null, null, (n) => (picked = n));
    view.querySelector<HTMLElement>('.filter button[data-node=b]')!.click();
    expect(picked).toBe('b');
  });
});

// req:web.views/backlog-taken@7225a6a8
describe('taken marks', () => {
  const snap = () => {
    const s = snapshot();
    s.backlog = ['a', 'b', 'c', 'd'].map((slug) => ({ slug, kind: 'idea', title: slug.toUpperCase(), nodes: ['n'], origin: null, blocks: [] }));
    return s;
  };
  const mark = (view: HTMLElement, slug: string) => view.querySelector<HTMLElement>(`.bi[data-slug=${slug}] .taken`)?.textContent ?? null;

  it('marks: a resolved by x, b proposed in y, c claimed, d unmarked', () => {
    const taken: Taken = {
      changes: [
        { id: 'x', state: 'main', items: ['a'], nodes: [] },
        { id: 'y', state: 'proposed', items: ['b'], nodes: ['web'] },
      ],
      claims: [{ item: 'c', nodes: [] }],
    };
    const view = renderBacklog(snap(), taken, null, () => {});
    expect(['a', 'b', 'c', 'd'].map((s) => mark(view, s))).toEqual(['resolved by x', 'proposed in y', 'claimed', null]);
  });

  it('a change on main wins over proposed work, and proposed work over a bare claim', () => {
    const taken: Taken = {
      changes: [
        { id: 'y', state: 'proposed', items: ['a', 'b'], nodes: [] },
        { id: 'x', state: 'main', items: ['a'], nodes: [] },
      ],
      claims: [{ item: 'a', nodes: [] }, { item: 'b', nodes: [] }],
    };
    const view = renderBacklog(snap(), taken, null, () => {});
    expect([mark(view, 'a'), mark(view, 'b')]).toEqual(['resolved by x', 'proposed in y']);
  });

  it('before the implementation state: the backlog is drawn, unmarked', () => {
    const view = renderBacklog(snap(), null, null, () => {});
    expect(slugs(view)).toEqual(['a', 'b', 'c', 'd']);
    expect(view.querySelector('.taken')).toBeNull();
  });
});

// req:web/propose-from-backlog@9b8191c8
describe('proposals', () => {
  const settle = async () => {
    for (let i = 0; i < 4; i++) await Promise.resolve();
  };
  const proposal = (over: Partial<Proposal> = {}): Proposal => ({ id: 'p1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', slug: 'export-csv', title: 'Export CSV', body: 'Let me export.', nodes: ['a'], mine: true, ...over });
  const proposing = (over: Partial<Proposing> = {}): Proposing => ({ proposals: [], offline: false, onPropose: vi.fn(() => Promise.resolve(null)), onWithdraw: vi.fn(() => Promise.resolve(null)), ...over });
  const openForm = (view: HTMLElement) => {
    // A checkbox fires change only when it's in the document.
    document.body.replaceChildren(view);
    view.querySelector<HTMLButtonElement>('button.propose')!.click();
    return view.querySelector<HTMLElement>('.fbform.proposal')!;
  };
  const fill = (form: HTMLElement, title: string, body: string, nodes: string[]) => {
    const t = form.querySelector<HTMLInputElement>('input[name=title]')!;
    t.value = title;
    t.dispatchEvent(new Event('input'));
    const b = form.querySelector('textarea')!;
    b.value = body;
    b.dispatchEvent(new Event('input'));
    for (const n of nodes) form.querySelector<HTMLInputElement>(`.picker input[value="${n}"]`)!.click();
  };
  beforeEach(() => drafts.clear());

  it("proposed: one post of the title, intent and picked components for two quick submits, then the form closes", async () => {
    let done: (v: string | null) => void = () => undefined;
    const onPropose = vi.fn(() => new Promise<string | null>((go) => (done = go)));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose }));
    const form = openForm(view);
    // The picker offers the graph's components.
    expect([...form.querySelectorAll<HTMLInputElement>('.picker input')].map((i) => i.value)).toEqual(['root', 'a', 'b']);
    fill(form, 'Export CSV', 'Let me export.', ['a']);
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onPropose).toHaveBeenCalledTimes(1);
    expect(onPropose).toHaveBeenCalledWith('Export CSV', 'Let me export.', ['a']);
    done(null);
    await settle();
    expect(view.querySelector('.fbform.proposal')).toBeNull();
  });

  it('Proposed lists each proposal with its title, components, proposer and the note, between Waiting on you and the columns', () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ proposals: [proposal()] }));
    const row = view.querySelector<HTMLElement>('.proposed')!;
    const p = row.querySelector<HTMLElement>('.prop[data-id=p1]')!;
    expect(p.textContent).toContain('Export CSV');
    expect([...p.querySelectorAll('.pill')].map((x) => x.textContent)).toEqual(['a']);
    expect(p.textContent).toContain('alice');
    expect(p.textContent).not.toContain('alice/web');
    expect(p.textContent).toContain('2026-10-27 10:00 UTC');
    expect(p.textContent).toContain('next build files it');
    const order = [...view.querySelectorAll('.waiting, .proposed, .col')].map((e) => e.className.split(' ').find((c) => ['waiting', 'proposed', 'col'].includes(c)));
    expect(order.slice(0, 3)).toEqual(['waiting', 'proposed', 'col']);
  });

  it('title taken: a refusal keeps the form open with its reason', async () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose: () => Promise.resolve('That title is taken: choose another.') }));
    const form = openForm(view);
    fill(form, 'Export CSV', '', []);
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(view.querySelector('.fbform.proposal .err')?.textContent).toBe('That title is taken: choose another.');
    expect(view.querySelector<HTMLInputElement>('.fbform.proposal input[name=title]')?.value).toBe('Export CSV');
  });

  it('sends nothing without a title', async () => {
    const onPropose = vi.fn(() => Promise.resolve(null));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose }));
    const form = openForm(view);
    fill(form, '  ', 'Body', []);
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(onPropose).not.toHaveBeenCalled();
    expect(form.querySelector('.err')?.textContent).toContain('title');
  });

  it('the picker takes at most 10 components', () => {
    const s = snapshot();
    const many = Array.from({ length: 12 }, (_, i) => `n${i}`);
    s.spec = { view: 'spec', nodes: many.map((n) => ({ ...s.spec.nodes[0], node: n })) };
    const form = openForm(renderBacklog(s, null, null, () => {}, proposing()));
    for (const n of many.slice(0, 10)) form.querySelector<HTMLInputElement>(`.picker input[value="${n}"]`)!.click();
    const boxes = [...form.querySelectorAll<HTMLInputElement>('.picker input')];
    expect(boxes.filter((b) => b.checked)).toHaveLength(10);
    expect(boxes.filter((b) => !b.checked).every((b) => b.disabled)).toBe(true);
  });

  it('what was typed survives a redraw', () => {
    fill(openForm(renderBacklog(snapshot(), null, null, () => {}, proposing())), 'Half', 'a thought', ['b']);
    const form = renderBacklog(snapshot(), null, null, () => {}, proposing()).querySelector<HTMLElement>('.fbform.proposal')!;
    expect(form.querySelector<HTMLInputElement>('input[name=title]')?.value).toBe('Half');
    expect(form.querySelector('textarea')?.value).toBe('a thought');
    expect(form.querySelector<HTMLInputElement>('.picker input[value="b"]')?.checked).toBe(true);
  });

  it('withdraw: only mine carries Withdraw, and a tap sends one withdrawal', () => {
    const onWithdraw = vi.fn(() => new Promise<string | null>(() => undefined));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onWithdraw, proposals: [proposal(), proposal({ id: 'p2', writer: 'bob/web', mine: false, title: 'Other' })] }));
    expect(view.querySelector('.prop[data-id=p2] button.withdraw')).toBeNull();
    const w = view.querySelector<HTMLButtonElement>('.prop[data-id=p1] button.withdraw')!;
    w.click();
    w.click();
    expect(onWithdraw).toHaveBeenCalledTimes(1);
    expect(onWithdraw).toHaveBeenCalledWith('p1');
  });

  it('a refused withdrawal says so under the proposal and can be tapped again', async () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onWithdraw: () => Promise.resolve('Not withdrawn (404).'), proposals: [proposal()] }));
    const w = view.querySelector<HTMLButtonElement>('.prop[data-id=p1] button.withdraw')!;
    w.click();
    await settle();
    expect(view.querySelector('.prop[data-id=p1] .err')?.textContent).toBe('Not withdrawn (404).');
    expect(w.disabled).toBe(false);
  });

  it('both disabled offline', () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ offline: true, proposals: [proposal()] }));
    expect(view.querySelector<HTMLButtonElement>('button.propose')!.disabled).toBe(true);
    expect(view.querySelector<HTMLButtonElement>('button.withdraw')!.disabled).toBe(true);
  });

  it('no Proposed row without proposals, or before the implementation state', () => {
    expect(renderBacklog(snapshot(), null, null, () => {}, proposing()).querySelector('.proposed')).toBeNull();
    expect(renderBacklog(snapshot(), null, null, () => {}, proposing({ proposals: null })).querySelector('.proposed')).toBeNull();
  });
});
