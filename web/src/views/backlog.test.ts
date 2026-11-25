import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Proposing, renderBacklog, renderBacklogItem, waiting } from './backlog';
import { drafts } from './forms';
import { snapshot } from './fixtures';
import { parseRoute } from './route';
import type { Proposal, Taken } from './types';

const slugs = (e: HTMLElement) => [...e.querySelectorAll<HTMLElement>('.bi')].map((b) => b.dataset.slug);

describe('backlog view', () => {
  // req:web.views/backlog-view@8c5ab166
  it('filter: only items on the chosen node are shown', () => {
    const view = renderBacklog(snapshot(), null, 'a', () => {});
    expect(slugs(view).sort()).toEqual(['crash', 'setup']);
    const all = renderBacklog(snapshot(), null, null, () => {});
    expect(slugs(all).sort()).toEqual(['crash', 'later', 'setup']);
  });

  // req:web.views/backlog-view@8c5ab166
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

  // req:web.views/backlog-view@8c5ab166
  it('a question blocking nothing yet still waits on you', () => {
    const snap = snapshot();
    snap.backlog.push({ slug: 'which', kind: 'question', title: 'Which?', nodes: ['b'], origin: null, blocks: [] });
    const view = renderBacklog(snap, null, null, () => {});
    const row = view.querySelector<HTMLElement>('.waiting')!;
    expect(slugs(row)).toEqual(['setup', 'which']);
    expect(row.querySelector<HTMLElement>('.bi[data-slug=which] .blocks')).toBeNull();
  });

  // req:web.views/backlog-view@8c5ab166
  it('groups the rest by kind, each card with title, nodes and slug', () => {
    const view = renderBacklog(snapshot(), null, null, () => {});
    const bugs = view.querySelector<HTMLElement>('.col[data-kind=bug]')!;
    const card = bugs.querySelector<HTMLElement>('.bi[data-slug=crash]')!;
    expect(card.querySelector('.ttl')?.textContent).toBe('It crashes');
    expect(card.querySelector('.pill')?.textContent).toBe('a');
    expect(card.querySelector('.slug')?.textContent).toBe('crash');
    expect(view.querySelector('.col[data-kind=idea] .bi[data-slug=later]')).not.toBeNull();
  });

  // req:web.views/backlog-view@8c5ab166
  it("an item opens its page: 'later' links to #/o/a/backlog/later, and that route shows the item", () => {
    const link = (slug: string) => `#/o/a/backlog/${slug}`;
    const view = renderBacklog(snapshot(), null, null, () => {}, undefined, link);
    const a = view.querySelector<HTMLAnchorElement>('.bi[data-slug=later] a.ttl')!;
    expect(a.getAttribute('href')).toBe('#/o/a/backlog/later');
    expect(view.querySelector('.waiting .bi[data-slug=setup] a.ttl')!.getAttribute('href')).toBe('#/o/a/backlog/setup');
    expect(parseRoute('#/o/a/backlog/later')).toMatchObject({ owner: 'o', repo: 'a', view: 'backlog', item: 'later' });
    const page = renderBacklogItem(snapshot(), 'setup', null, '#/o/a/backlog');
    expect(page.querySelector('h1')!.textContent).toBe('Create the account');
    expect(page.textContent).toContain('human-task');
    expect(page.textContent).toContain('c/a');
  });

  // req:web.views/backlog-view@8c5ab166
  it('a slug the backlog does not hold says so and links to the Backlog', () => {
    const page = renderBacklogItem(snapshot(), 'gone', null, '#/o/a/backlog');
    expect(page.textContent).toContain("isn't in the backlog");
    expect(page.querySelector('a')!.getAttribute('href')).toBe('#/o/a/backlog');
  });

  // req:web.views/backlog-view@8c5ab166
  it('picking a node pill filters', () => {
    let picked: string | null = 'x';
    const view = renderBacklog(snapshot(), null, null, (n) => (picked = n));
    view.querySelector<HTMLElement>('.filter button[data-node=b]')!.click();
    expect(picked).toBe('b');
  });

  // req:web.views/backlog-view@8c5ab166
  it('other kinds and the filter: Bugs, Deferred, Ideas, then each other kind; the filter lists all and the sorted components, Waiting on you included', () => {
    const snap = snapshot();
    snap.backlog = [
      { slug: 'crash', kind: 'bug', title: 'It crashes', nodes: ['b'], origin: null, blocks: [] },
      { slug: 'later', kind: 'idea', title: 'Later', nodes: ['a'], origin: null, blocks: [] },
      { slug: 'tidy', kind: 'chore', title: 'Tidy up', nodes: ['a'], origin: null, blocks: [] },
      { slug: 'which', kind: 'question', title: 'Which?', nodes: ['b'], origin: null, blocks: [] },
    ];
    const picked: (string | null)[] = [];
    const view = renderBacklog(snap, null, null, (n) => picked.push(n));
    const heads = [...view.querySelectorAll<HTMLElement>('.col')].map((c) => c.querySelector('.colh')!.childNodes[1].textContent);
    expect(heads).toEqual(['Bugs', 'Deferred', 'Ideas', 'chore']);
    expect([...view.querySelectorAll('.filter button')].map((b) => b.textContent)).toEqual(['all', 'a', 'b']);
    view.querySelector<HTMLElement>('.filter button[data-node=a]')!.click();
    expect(picked).toEqual(['a']);
    const onA = renderBacklog(snap, null, 'a', () => {});
    expect(onA.querySelector('.waiting')).toBeNull();
    expect(slugs(onA.querySelector<HTMLElement>('.col[data-kind=bug]')!)).toEqual([]);
    expect(slugs(onA)).toEqual(['later', 'tidy']);
    expect(slugs(view.querySelector<HTMLElement>('.waiting')!)).toEqual(['which']);
  });
});

describe('taken marks', () => {
  const snap = () => {
    const s = snapshot();
    s.backlog = ['a', 'b', 'c', 'd'].map((slug) => ({ slug, kind: 'idea', title: slug.toUpperCase(), nodes: ['n'], origin: null, blocks: [] }));
    return s;
  };
  const mark = (view: HTMLElement, slug: string) => view.querySelector<HTMLElement>(`.bi[data-slug=${slug}] .taken`)?.textContent ?? null;

  // req:web.views/backlog-taken@7225a6a8
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

  // req:web.views/backlog-taken@7225a6a8
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

  // req:web.views/backlog-taken@7225a6a8
  it('before the implementation state: the backlog is drawn, unmarked', () => {
    const view = renderBacklog(snap(), null, null, () => {});
    expect(slugs(view)).toEqual(['a', 'b', 'c', 'd']);
    expect(view.querySelector('.taken')).toBeNull();
  });
});

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
  const fill = (form: HTMLElement, title: string, body: string) => {
    const t = form.querySelector<HTMLInputElement>('input[name=title]')!;
    t.value = title;
    t.dispatchEvent(new Event('input'));
    const b = form.querySelector('textarea')!;
    b.value = body;
    b.dispatchEvent(new Event('input'));
  };
  beforeEach(() => drafts.clear());

  // req:web.views/propose-from-backlog@6d9d0fbe
  it("proposed: one post of the title and intent for two quick submits, then the form closes", async () => {
    let done: (v: string | null) => void = () => undefined;
    const onPropose = vi.fn(() => new Promise<string | null>((go) => (done = go)));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose }));
    const form = openForm(view);
    // The draft that takes the item up decides its components: the form has no picker.
    expect(form.querySelector('.picker, input[type=checkbox]')).toBeNull();
    fill(form, 'Export CSV', 'Let me export.');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onPropose).toHaveBeenCalledTimes(1);
    expect(onPropose).toHaveBeenCalledWith('Export CSV', 'Let me export.');
    done(null);
    await settle();
    expect(view.querySelector('.fbform.proposal')).toBeNull();
    // The view drawn with the implementation state that lists it.
    const after = renderBacklog(snapshot(), null, null, () => {}, proposing({ proposals: [proposal()] }));
    expect(after.querySelector('.fbform.proposal')).toBeNull();
    const listed = after.querySelector<HTMLElement>('.proposed .prop[data-id=p1]')!;
    expect(listed.textContent).toContain('Export CSV');
    expect(listed.textContent).toContain('alice');
    expect(listed.textContent).toContain('next build files it');
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('Proposed lists each proposal with its title, proposer, time and the note, between Waiting on you and the columns', () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ proposals: [proposal()] }));
    const row = view.querySelector<HTMLElement>('.proposed')!;
    const p = row.querySelector<HTMLElement>('.prop[data-id=p1]')!;
    expect(p.textContent).toContain('Export CSV');
    expect(p.textContent).toContain('alice');
    expect(p.textContent).not.toContain('alice/web');
    expect(p.textContent).toContain('2026-10-27 10:00 UTC');
    expect(p.textContent).toContain('next build files it');
    const order = [...view.querySelectorAll('.waiting, .proposed, .col')].map((e) => e.className.split(' ').find((c) => ['waiting', 'proposed', 'col'].includes(c)));
    expect(order.slice(0, 3)).toEqual(['waiting', 'proposed', 'col']);
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('title taken: a refusal keeps the form open with its reason', async () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose: () => Promise.resolve('That title is taken: choose another.') }));
    const form = openForm(view);
    fill(form, 'Export CSV', '');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(view.querySelector('.fbform.proposal .err')?.textContent).toBe('That title is taken: choose another.');
    expect(view.querySelector<HTMLInputElement>('.fbform.proposal input[name=title]')?.value).toBe('Export CSV');
    // A call that fails says the proposal wasn't sent, and keeps the form and its text.
    drafts.clear();
    const failed = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose: () => Promise.reject(new Error('down')) }));
    const again = openForm(failed);
    fill(again, 'Export CSV', 'Let me export.');
    again.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(failed.querySelector('.fbform.proposal .err')?.textContent).toBe('Not sent: offline.');
    expect(failed.querySelector<HTMLInputElement>('.fbform.proposal input[name=title]')?.value).toBe('Export CSV');
    expect(failed.querySelector<HTMLTextAreaElement>('.fbform.proposal textarea')?.value).toBe('Let me export.');
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('sends nothing without a title', async () => {
    const onPropose = vi.fn(() => Promise.resolve(null));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onPropose }));
    const form = openForm(view);
    fill(form, '  ', 'Body');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(onPropose).not.toHaveBeenCalled();
    expect(form.querySelector('.err')?.textContent).toBe('Give it a title first.');
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('what was typed survives a redraw', () => {
    fill(openForm(renderBacklog(snapshot(), null, null, () => {}, proposing())), 'Half', 'a thought');
    const form = renderBacklog(snapshot(), null, null, () => {}, proposing()).querySelector<HTMLElement>('.fbform.proposal')!;
    expect(form.querySelector<HTMLInputElement>('input[name=title]')?.value).toBe('Half');
    expect(form.querySelector('textarea')?.value).toBe('a thought');
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
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

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('a refused withdrawal says so under the proposal and can be tapped again', async () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onWithdraw: () => Promise.resolve('Not withdrawn (404).'), proposals: [proposal()] }));
    const w = view.querySelector<HTMLButtonElement>('.prop[data-id=p1] button.withdraw')!;
    w.click();
    await settle();
    expect(view.querySelector('.prop[data-id=p1] .err')?.textContent).toBe('Not withdrawn (404).');
    expect(w.disabled).toBe(false);
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('refusals: a refused withdrawal shows its reason and a second tap calls withdraw again; a failed one says offline', async () => {
    const onWithdraw = vi.fn(() => Promise.resolve('Not withdrawn (500).'));
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ onWithdraw, proposals: [proposal()] }));
    const w = view.querySelector<HTMLButtonElement>('.prop[data-id=p1] button.withdraw')!;
    w.click();
    await settle();
    expect(view.querySelector('.prop[data-id=p1] .err')?.textContent).toBe('Not withdrawn (500).');
    w.click();
    await settle();
    expect(onWithdraw.mock.calls).toEqual([['p1'], ['p1']]);
    const failed = renderBacklog(snapshot(), null, null, () => {}, proposing({ onWithdraw: () => Promise.reject(new Error('down')), proposals: [proposal()] }));
    const fw = failed.querySelector<HTMLButtonElement>('.prop[data-id=p1] button.withdraw')!;
    fw.click();
    await settle();
    expect(failed.querySelector('.prop[data-id=p1] .err')?.textContent).toBe('Not withdrawn: offline.');
    expect(fw.disabled).toBe(false);
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('draft kept: a redraw keeps the open form and its title, apart for each repository, until it is cancelled', () => {
    const of = (repo: string) => snapshot({ repo });
    fill(openForm(renderBacklog(of('o/a'), null, null, () => {}, proposing())), 'Export', '');
    const redrawn = renderBacklog(of('o/a'), null, null, () => {}, proposing());
    expect(redrawn.querySelector<HTMLInputElement>('.fbform.proposal input[name=title]')?.value).toBe('Export');
    const other = renderBacklog(of('o/b'), null, null, () => {}, proposing());
    expect(other.querySelector('.fbform.proposal')).toBeNull();
    expect(other.querySelector('button.propose')).not.toBeNull();
    redrawn.querySelector<HTMLButtonElement>('.fbform.proposal button.cancel')!.click();
    expect(renderBacklog(of('o/a'), null, null, () => {}, proposing()).querySelector('.fbform.proposal')).toBeNull();
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('both disabled offline', () => {
    const view = renderBacklog(snapshot(), null, null, () => {}, proposing({ offline: true, proposals: [proposal()] }));
    expect(view.querySelector<HTMLButtonElement>('button.propose')!.disabled).toBe(true);
    expect(view.querySelector<HTMLButtonElement>('button.withdraw')!.disabled).toBe(true);
  });

  // req:web.views/propose-from-backlog@6d9d0fbe
  it('no Proposed row without proposals, or before the implementation state', () => {
    expect(renderBacklog(snapshot(), null, null, () => {}, proposing()).querySelector('.proposed')).toBeNull();
    expect(renderBacklog(snapshot(), null, null, () => {}, proposing({ proposals: null })).querySelector('.proposed')).toBeNull();
  });
});
