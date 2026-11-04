import { beforeEach, describe, expect, it, vi } from 'vitest';
import { drafts } from './forms';
import { cannotAct, renderPrList, renderReview, specPath } from './pr';
import { branches, review } from '../test/fixtures';
import type { Target } from './route';
import type { BranchReview } from './types';

const opts = (over = {}) => ({ offline: false, accepted: false, onAct: vi.fn(), fullHref: (p: string) => `#full/${p}`, ...over });
const link = (t: Target) => `#b/${t.branch}`;

// req:web/pr-document@0e70b0ab
describe('Document tab', () => {
  it('modified requirement, asset and decision: n/r modified with quickly inserted, m.html added in an empty sandbox, d added with its text', () => {
    const view = renderReview(review(), opts());
    const r = view.querySelector<HTMLElement>('.req[data-el="n/r"]')!;
    expect(r.querySelector('.chip')?.textContent).toBe('Modified');
    expect(r.querySelector('ins')?.textContent).toContain('quickly');
    expect(r.textContent).toContain('It SHALL do r');

    const asset = view.querySelector<HTMLElement>('.req[data-el="m.html"]')!;
    expect(asset.querySelector('.chip')?.textContent).toBe('Added');
    const frame = asset.querySelector('iframe')!;
    expect(frame.getAttribute('sandbox')).toBe('');
    expect(frame.getAttribute('srcdoc')).toBe('<p>mock</p>');

    const dec = view.querySelector<HTMLElement>('.dec[data-name=d]')!;
    expect(dec.textContent).toContain('added');
    expect(dec.textContent).toContain('We decided.');
  });

  it('gives each changed element a couple of dimmed lines of context from its file', () => {
    const after = '### Design\n\nSome design.\n\n### Requirement: r\nIt SHALL do r quickly.\n\n### Requirement: s\nIt SHALL do s.\n';
    const view = renderReview(review({ documents: [{ path: 'specs/n/SPEC.md', before: null, after }] }), opts());
    const ctx = [...view.querySelectorAll('.req[data-el="n/r"] .ctx')].map((c) => c.textContent);
    expect(ctx).toEqual(['Some design.\n### Requirement: r', '### Requirement: s\nIt SHALL do s.']);
  });

  it('links to the files: clm, n and m.n open their own SPEC.md in full', () => {
    const r = review({
      documents: [
        { path: 'specs/SPEC.md', before: 'a', after: 'b' },
        { path: 'specs/n/SPEC.md', before: 'a', after: 'b' },
        { path: 'specs/m/n/SPEC.md', before: 'a', after: 'b' },
      ],
    });
    r.nodes = ['clm', 'n', 'm.n'].map((id) => ({ ...r.nodes[0], id }));
    expect(specPath('clm')).toBe('specs/SPEC.md');
    expect(specPath('m.n')).toBe('specs/m/n/SPEC.md');
    const view = renderReview(r, opts());
    const links = [...view.querySelectorAll('.node-changes a.open-full')].map((a) => a.getAttribute('href'));
    expect(links).toEqual(['#full/specs/SPEC.md', '#full/specs/n/SPEC.md', '#full/specs/m/n/SPEC.md']);
  });

  it('shows Contract, Design and frontmatter changes, and the backlog edits', () => {
    const r = review({ backlog: [{ slug: 'crash', status: 'removed', before: 'It crashes.', after: null }] });
    r.nodes = [{ ...r.nodes[0], contract: { before: 'Old.', after: 'New.' }, design: { before: null, after: 'D.' }, frontmatter: { before: 'a', after: 'b' } }];
    const view = renderReview(r, opts());
    expect(view.querySelector('.req[data-el="n:contract"] .chip')?.textContent).toBe('Modified');
    expect(view.querySelector('.req[data-el="n:design"] .chip')?.textContent).toBe('Added');
    expect(view.querySelector('.req[data-el="n:frontmatter"]')).not.toBeNull();
    expect(view.querySelector('.dec[data-slug=crash]')?.textContent).toContain('removed');
  });
});

// req:web/review-intent-first@563a463a
describe('Intent first', () => {
  it('intent first: the Intent section comes before the node changes, beside Approve', () => {
    const view = renderReview(review(), opts());
    document.body.replaceChildren(view);
    const head = view.querySelector<HTMLElement>('.prhead')!;
    expect(head.querySelector('.intent')?.textContent).toContain('Make b better.');
    expect(head.querySelector('.intent')?.textContent).not.toContain('r changes.');
    expect(head.querySelector(':scope > .approve-bar button.approve')).not.toBeNull();
    const all = [...view.querySelectorAll('.intent, .node-changes')];
    expect(all.map((e) => e.className.split(' ').at(-1))).toEqual(['intent', 'node-changes']);
    expect(view.querySelector('.significant')).toBeNull();
    expect(view.textContent).not.toContain('Significant');
  });

  it('intent first without an Intent section: the whole intent, before the node changes', () => {
    for (const intent_section of [null, '']) {
      const change = { ...review().change!, intent: 'Just prose.', intent_section };
      const view = renderReview(review({ change }), opts());
      const head = view.querySelector<HTMLElement>('.prhead')!;
      expect(head.querySelector('.intent')?.textContent).toContain('Just prose.');
      expect(head.querySelector(':scope > .approve-bar button.approve')).not.toBeNull();
      const all = [...view.querySelectorAll('.intent, .node-changes')];
      expect(all.map((e) => e.className.split(' ').at(-1))).toEqual(['intent', 'node-changes']);
    }
  });
});

// req:web/spec-branch-review@a7d7ac89
describe('Needs review', () => {
  it("needs review without a change: the review's top-level review decides, not the change", () => {
    const meta = (r: BranchReview) => renderReview(r, opts()).querySelector('.meta')!.textContent;
    const arch = meta(review({ branch: 'spec/arch', review: true, change: null }));
    const plain = meta(review({ branch: 'spec/plain', review: false }));
    expect(review({ review: false }).change).not.toBeNull();
    expect(arch).toContain('needs review');
    expect(plain).not.toContain('review');
    const chip = (r: BranchReview) => renderReview(r, opts()).querySelector('.meta .chip.hard');
    expect(chip(review({ review: true, change: null }))?.textContent).toBe('needs review');
    expect(chip(review({ review: false }))).toBeNull();
    expect(arch).not.toContain('gate');
  });
});

// req:web/spec-branch-review@a7d7ac89
describe('spec branches', () => {
  it('branch without a page: both listed, spec/c with #7 and its title, spec/d with no page', () => {
    const list = renderPrList(branches, link);
    const rows = [...list.querySelectorAll<HTMLElement>('.prl')];
    expect(rows.map((r) => r.dataset.branch)).toEqual(['spec/c', 'spec/d']);
    expect(rows[0].querySelector<HTMLAnchorElement>('a.open')?.getAttribute('href')).toBe('#b/spec/c');
    expect(rows[0].textContent).toContain('#7');
    expect(rows[0].textContent).toContain('Change b');
    expect(rows[0].querySelector<HTMLAnchorElement>('a.page')?.href).toBe('https://github.com/o/a/pull/7');
    expect(rows[1].textContent).not.toContain('#');
    expect(rows[1].querySelector('a.page')).toBeNull();
  });

  it('says when no spec branch is open', () => {
    expect(renderPrList([], link).textContent).toContain('No spec branches');
  });

  it('approved by: the header names the approval with its time and says not checked; no labels', () => {
    const view = renderReview(review({ approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }], checked: false }), opts());
    const head = view.querySelector<HTMLElement>('.head')!;
    expect(head.textContent).toContain('Approved by clunny/web');
    expect(head.textContent).toContain('2026-10-27 10:00 UTC');
    expect(head.textContent).toContain('not checked');
    expect(head.querySelector<HTMLAnchorElement>('a.page')?.href).toBe('https://github.com/o/a/pull/7');
    expect(head.querySelector('.label, .labels')).toBeNull();
    expect(head.textContent).not.toMatch(/spec-approved|label/i);
  });

  it('says not approved and checked, and shows no page link without a page', () => {
    const head = renderReview(review({ page: null }), opts()).querySelector<HTMLElement>('.head')!;
    expect(head.textContent).toContain('not approved');
    expect([...head.querySelectorAll('.chip')].map((c) => c.textContent)).toContain('checked');
    expect(head.querySelector('a.page')).toBeNull();
  });
});

// req:web/approve-button@ff59f9d1
describe('Approve intent', () => {
  it('one request, then approved: a second tap while the first is in flight sends nothing', () => {
    const o = opts();
    const button = renderReview(review(), o).querySelector<HTMLButtonElement>('button.approve')!;
    button.click();
    button.click();
    expect(o.onAct).toHaveBeenCalledTimes(1);
    expect(button.disabled).toBe(true);
    const after = renderReview(review(), opts({ accepted: true })).querySelector<HTMLButtonElement>('button.approve')!;
    expect(after.textContent).toBe('Approved');
    expect(after.disabled).toBe(true);
  });

  it('disabled: without a fingerprint, once an approval shows, and offline', () => {
    for (const [r, o] of [
      [review({ fingerprint: null }), opts()],
      [review({ approvals: [{ writer: 'a/web', at: '2026-10-27T10:00:00Z' }] }), opts()],
      [review(), opts({ offline: true })],
    ] as const) {
      expect(renderReview(r, o).querySelector<HTMLButtonElement>('button.approve')!.disabled).toBe(true);
    }
    expect(renderReview(review(), opts()).querySelector<HTMLButtonElement>('button.approve')!.disabled).toBe(false);
  });
});

// req:web/approve-by-branch@971e758a
describe('what Approve may send', () => {
  it('no fingerprint: Approve is disabled', () => {
    expect(cannotAct(review({ fingerprint: null }), false, false)).not.toBeNull();
    expect(renderReview(review({ fingerprint: null }), opts()).querySelector<HTMLButtonElement>('button.approve')!.disabled).toBe(true);
  });

  it('an approval on screen comes from the review, not from a label', () => {
    expect(cannotAct(review(), false, false)).toBeNull();
    expect(cannotAct(review({ approvals: [{ writer: 'a/web', at: 'x' }] }), false, false)).toBe('Approved');
  });
});

// req:web/spec-branch-review@a7d7ac89
describe('no review list', () => {
  it('the PR list shows the spec branches only: no To review list and no Mark reviewed', () => {
    const list = renderPrList(branches, link);
    expect(list.querySelector('.to-review')).toBeNull();
    expect(list.textContent).not.toContain('To review');
    expect([...list.querySelectorAll('button')].map((b) => b.textContent)).not.toContain('Mark reviewed');
    expect([...list.querySelectorAll<HTMLElement>('.prl')].map((r) => r.dataset.branch)).toEqual(branches.map((b) => b.branch));
  });
});

// req:web/signed-approvals@9bdf6473
describe('signed repositories', () => {
  const approvalMarks = (view: HTMLElement) => [...view.querySelectorAll('.head .sig')].map((c) => c.textContent);

  it('signed repository: Approve is disabled, never reads Approved, and the note names clm approve', () => {
    const o = opts();
    const r = review({ branch: 'spec/x', signed: true, approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z', signed: false }] });
    const view = renderReview(r, o);
    const button = view.querySelector<HTMLButtonElement>('button.approve')!;
    expect(button.disabled).toBe(true);
    expect(button.textContent).not.toContain('Approved');
    const note = view.querySelector<HTMLElement>('.approve-bar .signed-note')!;
    expect(note.textContent).toContain("signed with the reviewer's SSH key");
    expect(note.querySelector('code')?.textContent).toBe('clm approve spec/x');
    expect(note.querySelector('button.copy')).not.toBeNull();
    const head = view.querySelector<HTMLElement>('.head')!;
    expect(head.textContent).toContain('clunny/web');
    expect(approvalMarks(view)).toEqual(["unsigned, doesn't count"]);
    expect(head.textContent).toContain('no signed approval');
    // Approvals are drawn neutral, not green, in a signed repository.
    expect([...head.querySelectorAll('.chip.ok')].some((c) => c.textContent?.includes('clunny/web'))).toBe(false);
    button.click();
    expect(o.onAct).not.toHaveBeenCalled();
  });

  it('signed approval: marked signed, and no "no signed approval"', () => {
    const view = renderReview(review({ branch: 'spec/x', signed: true, approvals: [{ writer: 'dana/9f9f9f9f', at: '2026-10-27T10:00:00Z', signed: true }] }), opts());
    expect(approvalMarks(view)).toEqual(['signed']);
    expect(view.querySelector('.head')?.textContent).not.toContain('no signed approval');
    expect(view.querySelector('button.approve')?.textContent).toBe('Approve intent');
  });

  it('signed with no approvals says not approved, and the note shows offline too', () => {
    const view = renderReview(review({ branch: 'spec/x', signed: true }), opts({ offline: true }));
    expect(view.querySelector('.head')?.textContent).toContain('not approved');
    expect(view.querySelector('.head')?.textContent).not.toContain('no signed approval');
    expect(view.querySelector('.signed-note code')?.textContent).toBe('clm approve spec/x');
  });

  it('copy puts the command on the clipboard', () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    const view = renderReview(review({ branch: 'spec/x', signed: true }), opts());
    view.querySelector<HTMLButtonElement>('.signed-note button.copy')!.click();
    expect(writeText).toHaveBeenCalledWith('clm approve spec/x');
  });

  it('unsigned repository: Approve enabled, no note and no marks, with signed false or absent', () => {
    const absent = review({ branch: 'spec/x' });
    delete (absent as { signed?: boolean }).signed;
    for (const r of [review({ branch: 'spec/x', signed: false }), absent]) {
      const view = renderReview(r, opts());
      expect(view.querySelector<HTMLButtonElement>('button.approve')!.disabled).toBe(false);
      expect(view.querySelector('.signed-note')).toBeNull();
      expect(approvalMarks(view)).toEqual([]);
    }
    const approved = renderReview(review({ signed: false, approvals: [{ writer: 'a/web', at: '2026-10-27T10:00:00Z', signed: false }] }), opts());
    expect(approvalMarks(approved)).toEqual([]);
    expect(approved.querySelector('.head')?.textContent).toContain('Approved by a/web');
  });
});

const settle = async () => {
  for (let i = 0; i < 4; i++) await Promise.resolve();
};
const typeIn = (form: HTMLElement, text: string) => {
  const t = form.querySelector('textarea')!;
  t.value = text;
  t.dispatchEvent(new Event('input'));
};

// req:web/request-changes@8ce19ec8
describe('Request changes', () => {
  const held = (over = {}) => ({ id: 'q1', writer: 'alice/web', at: '2026-10-27T12:00:00Z', fingerprint: 'f1', text: 'Split it', current: true, held: true, ...over });
  const button = (view: HTMLElement) => view.querySelector<HTMLButtonElement>('.approve-bar button.request')!;
  beforeEach(() => drafts.clear());

  it('requested: beside Approve, two quick submits send once, and the header says changes requested by alice', async () => {
    let done: (v: string | null) => void = () => undefined;
    const onRequest = vi.fn(() => new Promise<string | null>((go) => (done = go)));
    const view = renderReview(review({ branch: 'spec/x', signed: true }), opts({ onRequest }));
    expect(button(view).disabled).toBe(false);
    button(view).click();
    const form = view.querySelector<HTMLElement>('.approve-bar .fbform')!;
    typeIn(form, 'Split it');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onRequest).toHaveBeenCalledTimes(1);
    expect(onRequest).toHaveBeenCalledWith('Split it');
    done(null);
    await settle();

    const after = renderReview(review({ branch: 'spec/x', signed: true, change_requests: [held()] }), opts());
    const h = after.querySelector<HTMLElement>('.head .changes-held')!;
    expect(h.textContent).toContain('Changes requested by alice');
    expect(h.textContent).not.toContain('alice/web');
    expect(h.textContent).toContain('2026-10-27 12:00 UTC');
    expect(h.textContent).toContain('Split it');
    expect(h.textContent).toContain("won't land until the author pushes a change or alice approves it");
    // A held request is the header's, not a comment under the intent.
    expect(after.querySelector('.intent .fb')).toBeNull();
  });

  it('disabled: without a fingerprint and offline', () => {
    expect(button(renderReview(review({ fingerprint: null }), opts())).disabled).toBe(true);
    expect(button(renderReview(review(), opts({ offline: true }))).disabled).toBe(true);
  });

  it('enabled in a signed repository, after an approval and after an earlier request', () => {
    for (const r of [
      review({ signed: true }),
      review({ approvals: [{ writer: 'a/web', at: '2026-10-27T10:00:00Z' }] }),
      review({ change_requests: [held()] }),
    ]) {
      expect(button(renderReview(r, opts())).disabled).toBe(false);
    }
  });
});

// req:web/comment-on-review@1279617c
describe('comments', () => {
  const comment = (over = {}) => ({ id: 'c1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', fingerprint: 'f1', target: null, text: 'Looks right', current: true, ...over });
  const open = (root: HTMLElement, at: string) => {
    root.querySelector<HTMLButtonElement>(`${at} button.comment`)!.click();
    return root.querySelector<HTMLElement>(`${at} .fbform`)!;
  };
  beforeEach(() => drafts.clear());

  it('comment on a requirement: one post of the fingerprint, target and text, and the card shows the comment by alice', async () => {
    let done: (v: string | null) => void = () => undefined;
    const onComment = vi.fn(() => new Promise<string | null>((go) => (done = go)));
    const view = renderReview(review({ branch: 'spec/x' }), opts({ onComment }));
    const form = open(view, '.req[data-el="n/r"]');
    typeIn(form, ' Name the rule ');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onComment).toHaveBeenCalledTimes(1);
    expect(onComment).toHaveBeenCalledWith('n/r', 'Name the rule');
    done(null);
    await settle();
    expect(view.querySelector('.req[data-el="n/r"] .fbform')).toBeNull();

    const after = renderReview(review({ branch: 'spec/x', comments: [comment({ target: 'n/r', text: 'Name the rule' })] }), opts());
    const fb = after.querySelector<HTMLElement>('.req[data-el="n/r"] .fb')!;
    expect(fb.textContent).toContain('alice');
    expect(fb.textContent).not.toContain('alice/web');
    expect(fb.textContent).toContain('Name the rule');
    expect(fb.textContent).toContain('2026-10-27 10:00 UTC');
    expect(after.querySelector('.intent .fb')).toBeNull();
  });

  it('the intent carries Comment too, sending a null target', () => {
    const onComment = vi.fn(() => Promise.resolve(null));
    const view = renderReview(review(), opts({ onComment }));
    const form = open(view, '.intent');
    typeIn(form, 'Why now?');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onComment).toHaveBeenCalledWith(null, 'Why now?');
  });

  it('sends nothing for an empty text, and says so', async () => {
    const onComment = vi.fn(() => Promise.resolve(null));
    const view = renderReview(review(), opts({ onComment }));
    const form = open(view, '.intent');
    typeIn(form, '   ');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(onComment).not.toHaveBeenCalled();
    expect(form.querySelector('.err')?.textContent).toContain('Write something');
  });

  it('earlier version: the comment under the intent marked on an earlier version, the request under it marked since approved', () => {
    const r = review({
      fingerprint: 'f2',
      comments: [comment({ fingerprint: 'f1', current: false })],
      change_requests: [{ id: 'q1', writer: 'bob/web', at: '2026-10-27T11:00:00Z', fingerprint: 'f2', text: 'Split it', current: true, held: false }],
    });
    const items = [...renderReview(r, opts()).querySelectorAll<HTMLElement>('.intent .fb')];
    expect(items.map((i) => i.textContent?.includes('on an earlier version'))).toEqual([true, false]);
    expect(items[1].textContent).toContain('since approved');
    expect(items[1].textContent).toContain('bob');
    expect(items[1].textContent).toContain('Split it');
  });

  it('a comment on a requirement the review shows no card for is drawn under the intent', () => {
    const view = renderReview(review({ comments: [comment({ target: 'z/gone' })] }), opts());
    expect(view.querySelector('.intent .fb')?.textContent).toContain('Looks right');
    expect(view.querySelector('.intent .fb')?.textContent).toContain('on z/gone');
  });

  it("drafts are kept per review scope, so another repository's branch of the same name has its own", () => {
    typeIn(open(renderReview(review(), opts({ scope: '/api/repos/o/a/branches/spec/c' })), '.intent'), 'for a');
    expect(renderReview(review(), opts({ scope: '/api/repos/o/b/branches/spec/c' })).querySelector('.intent .fbform')).toBeNull();
    expect(renderReview(review(), opts({ scope: '/api/repos/o/a/branches/spec/c' })).querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('for a');
  });

  it('a review without comments or change_requests reads as having none', () => {
    const r = review();
    delete r.comments;
    delete r.change_requests;
    expect(renderReview(r, opts()).querySelectorAll('.fb')).toHaveLength(0);
  });

  it('a refusal shows its reason under the form and keeps it', async () => {
    const view = renderReview(review(), opts({ onComment: () => Promise.resolve('Not sent: the text was refused.') }));
    const form = open(view, '.intent');
    typeIn(form, 'x');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(view.querySelector('.intent .fbform .err')?.textContent).toBe('Not sent: the text was refused.');
    expect(view.querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('x');
  });

  it('what was typed survives a redraw', () => {
    const first = renderReview(review(), opts());
    typeIn(open(first, '.intent'), 'half a thought');
    const again = renderReview(review(), opts());
    expect(again.querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('half a thought');
  });

  it('disabled offline and without a fingerprint, on the intent and every card; none on a land', () => {
    for (const [r, o] of [
      [review({ fingerprint: null }), opts()],
      [review(), opts({ offline: true })],
    ] as const) {
      const buttons = [...renderReview(r, o).querySelectorAll<HTMLButtonElement>('button.comment')];
      expect(buttons.length).toBe(2);
      expect(buttons.every((b) => b.disabled)).toBe(true);
    }
    expect([...renderReview(review(), opts()).querySelectorAll<HTMLButtonElement>('button.comment')].every((b) => !b.disabled)).toBe(true);
  });
});
