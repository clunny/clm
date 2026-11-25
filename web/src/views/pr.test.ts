import { beforeEach, describe, expect, it, vi } from 'vitest';
import { drafts } from './forms';
import { cannotAct, renderPrList, renderReview, specPath } from './pr';
import type { ReviewOptions } from './pr';
import { branches, review } from './fixtures';
import { parseRoute, reviewHref, reviewPath } from './route';
import type { Target } from './route';
import type { BranchReview, SpecBranch } from './types';

const opts = (over: Partial<ReviewOptions> = {}): ReviewOptions => ({
  offline: false,
  accepted: false,
  onAct: vi.fn(),
  fullHref: (p: string) => `#full/${p}`,
  onComment: vi.fn(async () => null),
  onRequest: vi.fn(async () => null),
  ...over,
});
const link = (t: Target) => `#b/${t.branch}`;

describe('Document tab', () => {
  // req:web.views/pr-document@3b6150b3
  it('modified requirement, asset and decision: n/r modified with quickly inserted, m.html added in an empty sandbox, d added with its text', () => {
    const view = renderReview(review(), opts());
    const r = view.querySelector<HTMLElement>('.req[data-el="n/r"]')!;
    expect(r.querySelector('.chip')?.textContent).toBe('Modified');
    expect(r.querySelector('ins')?.textContent).toContain('quickly');
    expect(r.textContent).toContain('It SHALL do r');
    expect(r.querySelector('ins')?.textContent).not.toContain('It SHALL do r');
    // Without what was inserted, the marked text reads as the old text.
    const marked = r.querySelector<HTMLElement>('.rb')!.cloneNode(true) as HTMLElement;
    marked.querySelectorAll('ins, .ctx').forEach((e) => e.remove());
    expect(marked.textContent!.replace(/\s+/g, ' ').replace(' .', '.').trim()).toBe('It SHALL do r.');

    const asset = view.querySelector<HTMLElement>('.req[data-el="m.html"]')!;
    expect(asset.querySelector('.chip')?.textContent).toBe('Added');
    const frame = asset.querySelector('iframe')!;
    expect(frame.getAttribute('sandbox')).toBe('');
    expect(frame.getAttribute('srcdoc')).toBe('<p>mock</p>');

    const dec = view.querySelector<HTMLElement>('.dec[data-name=d]')!;
    expect(dec.textContent).toContain('added');
    expect(dec.textContent).toContain('We decided.');
  });

  // req:web.views/pr-document@3b6150b3
  it('gives each changed element a couple of dimmed lines of context from its file', () => {
    const after = '### Design\n\nSome design.\n\n### Requirement: r\nIt SHALL do r quickly.\n\n### Requirement: s\nIt SHALL do s.\n';
    const view = renderReview(review({ documents: [{ path: 'specs/n/SPEC.md', before: null, after }] }), opts());
    const ctx = [...view.querySelectorAll('.req[data-el="n/r"] .ctx')].map((c) => c.textContent);
    expect(ctx).toEqual(['Some design.\n### Requirement: r', '### Requirement: s\nIt SHALL do s.']);
  });

  // req:web.views/pr-document@3b6150b3
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

  // req:web.views/pr-document@3b6150b3
  it('shows Contract, Design and frontmatter changes, and the backlog edits', () => {
    const r = review({ backlog: [{ slug: 'crash', status: 'removed', before: 'It crashes.', after: null }] });
    r.nodes = [{ ...r.nodes[0], contract: { before: 'Old.', after: 'New.' }, design: { before: null, after: 'D.' }, frontmatter: { before: 'a', after: 'b' } }];
    const view = renderReview(r, opts());
    expect(view.querySelector('.req[data-el="n:contract"] .chip')?.textContent).toBe('Modified');
    expect(view.querySelector('.req[data-el="n:design"] .chip')?.textContent).toBe('Added');
    expect(view.querySelector('.req[data-el="n:frontmatter"]')).not.toBeNull();
    expect(view.querySelector('.dec[data-slug=crash]')?.textContent).toContain('removed');
  });

  // req:web.views/pr-document@3b6150b3
  it('impact: components touched, requirements added and removed, Contracts and Designs, the files each linking in full; none for a review that changes no requirement', () => {
    const r = review({
      documents: [
        { path: 'specs/n/SPEC.md', before: 'a', after: 'b' },
        { path: 'specs/m/SPEC.md', before: 'a', after: 'b' },
      ],
    });
    const none = { added: [], modified: [], removed: [] };
    r.nodes = [
      {
        ...r.nodes[0],
        id: 'n',
        contract: { before: 'Old.', after: 'New.' },
        requirements: {
          added: [
            { id: 'n/p', before: null, after: 'P.' },
            { id: 'n/q', before: null, after: 'Q.' },
          ],
          modified: [],
          removed: [{ id: 'n/old', before: 'Old.', after: null }],
        },
        assets: none,
      },
      { ...r.nodes[0], id: 'm', frontmatter: { before: 'a', after: 'b' }, requirements: none, assets: none },
    ];
    const impact = (v: HTMLElement) => Object.fromEntries([...v.querySelectorAll('.aside .kv')].filter((k) => k.querySelector('b')).map((k) => [k.firstChild!.textContent, k.querySelector('b')!.textContent]));
    const view = renderReview(r, opts());
    expect(impact(view)).toEqual({ 'Components touched': '2', Requirements: '2 added, 1 removed', Contracts: '1', Designs: '0' });
    expect([...view.querySelectorAll('.aside a.mono')].map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['specs/n/SPEC.md', '#full/specs/n/SPEC.md'],
      ['specs/m/SPEC.md', '#full/specs/m/SPEC.md'],
    ]);
    const plain = review();
    plain.nodes = [{ ...plain.nodes[0], requirements: none }];
    expect(impact(renderReview(plain, opts())).Requirements).toBe('none');
  });
});

describe('Intent first', () => {
  // req:web.views/review-intent-first@26efd199
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

  // req:web.views/review-intent-first@26efd199
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

  // req:web.views/review-intent-first@26efd199
  it('no change: a branch with no change says so where the intent goes, beside Approve', () => {
    const view = renderReview(review({ change: null }), opts());
    const head = view.querySelector<HTMLElement>('.prhead')!;
    expect(head.querySelector('.intent .note')?.textContent).toBe('This change has no change.md.');
    expect(head.querySelector(':scope > .approve-bar button.approve')).not.toBeNull();
    const intent = view.querySelector('.intent')!;
    expect(intent.compareDocumentPosition(view.querySelector('.node-changes')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});

describe('Needs review', () => {
  // req:web.views/spec-branch-review@bf6312ca
  it("needs review without a change: the review's top-level review decides, not the change", () => {
    const meta = (r: BranchReview) => renderReview(r, opts()).querySelector('.meta')!.textContent;
    const arch = meta(review({ branch: 'spec/arch', review: true, change: null }));
    const plain = meta(review({ branch: 'spec/plain', review: false }));
    expect(review({ review: false }).change).not.toBeNull();
    expect(arch).toContain('needs review');
    expect(plain).not.toContain('review');
    const chip = (r: BranchReview) => renderReview(r, opts()).querySelector('.meta .chip.hard');
    expect(chip(review({ review: true, change: null }))?.textContent).toBe('needs review');
    expect(review({ review: true }).change).not.toBeNull();
    expect(chip(review({ review: true }))?.textContent).toBe('needs review');
    expect(chip(review({ review: false }))).toBeNull();
    expect(arch).not.toContain('gate');
  });
});

describe('spec branches', () => {
  // req:web.views/spec-branch-review@bf6312ca
  it('branch without a page: both listed, spec/a with #7 and "Add a", spec/b routed by its name as one segment, and its review shows no page link', () => {
    const given: SpecBranch[] = [
      { branch: 'spec/a', head_sha: 'h1', page: { number: 7, url: 'https://github.com/o/a/pull/7', title: 'Add a' } },
      { branch: 'spec/b', head_sha: 'h2', page: null },
    ];
    const list = renderPrList(given, (t) => reviewHref('o', 'a', t));
    const rows = [...list.querySelectorAll<HTMLElement>('.prl')];
    expect(rows.map((r) => r.dataset.branch)).toEqual(['spec/a', 'spec/b']);
    expect(rows.map((r) => r.querySelector('a.open')?.textContent)).toEqual(['spec/a', 'spec/b']);
    expect(rows[0].querySelector<HTMLAnchorElement>('a.open')?.getAttribute('href')).toBe('#/o/a/prs/spec%2Fa');
    expect(rows[0].textContent).toContain('#7');
    expect(rows[0].textContent).toContain('Add a');
    expect(rows[0].querySelector<HTMLAnchorElement>('a.page')?.href).toBe('https://github.com/o/a/pull/7');
    expect(rows[1].textContent).not.toContain('#');
    expect(rows[1].querySelector('a.page')).toBeNull();
    // Each row links by the link its caller gives for the branch.
    expect([...renderPrList(given, link).querySelectorAll<HTMLAnchorElement>('.prl a.open')].map((a) => a.getAttribute('href'))).toEqual(['#b/spec/a', '#b/spec/b']);
    const open = rows[1].querySelector<HTMLAnchorElement>('a.open')!.getAttribute('href')!;
    expect(open).toBe('#/o/a/prs/spec%2Fb');
    const route = parseRoute(open);
    expect(route.review).toEqual({ branch: 'spec/b' });
    expect(reviewPath(route.review!)).toBe('branches/spec/b');
    const head = renderReview(review({ branch: 'spec/b', page: null }), opts()).querySelector<HTMLElement>('.head')!;
    expect(head.querySelector('h1')?.textContent).toBe('spec/b');
    expect(head.querySelector('a.page')).toBeNull();
  });

  // req:web.views/spec-branch-review@bf6312ca
  it('says when no spec branch is open', () => {
    expect(renderPrList([], link).textContent).toContain('No spec branches');
  });

  // req:web.views/spec-branch-review@bf6312ca
  it('approved by: the header names the approval with its time and says not checked; no labels', () => {
    const view = renderReview(review({ branch: 'spec/x', approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }], checked: false }), opts());
    const head = view.querySelector<HTMLElement>('.head')!;
    expect(head.querySelector('h1')?.textContent).toBe('spec/x');
    const two = renderReview(review({ approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }, { writer: 'dana/web', at: '2026-10-28T09:30:00Z' }] }), opts());
    const shown = two.querySelector<HTMLElement>('.head .meta')!.textContent!;
    expect(shown).toContain('Approved by clunny/web · 2026-10-27 10:00 UTC');
    expect(shown).toContain('Approved by dana/web · 2026-10-28 09:30 UTC');
    expect(head.textContent).toContain('Approved by clunny/web');
    expect(head.textContent).toContain('2026-10-27 10:00 UTC');
    expect(head.textContent).toContain('not checked');
    expect(head.querySelector<HTMLAnchorElement>('a.page')?.href).toBe('https://github.com/o/a/pull/7');
    expect(head.querySelector('.label, .labels')).toBeNull();
    expect(head.textContent).not.toMatch(/spec-approved|label/i);
  });

  // req:web.views/spec-branch-review@bf6312ca
  it("a branch that cuts: Cuts lists the review's lines under the header, above its components; none without cuts", () => {
    const lines = ['specs/n/SPEC.md: gate/cut: requirement n/r is gone', 'specs/n/SPEC.md: gate/cut: scenario n/s: one is gone'];
    const view = renderReview(review({ branch: 'spec/x', cuts: lines }), opts());
    const cuts = view.querySelector<HTMLElement>('.cuts')!;
    expect(cuts.querySelector('h3')?.textContent).toBe('Cuts');
    expect([...cuts.querySelectorAll('.cut')].map((c) => c.textContent)).toEqual(lines);
    // In the header, under its chips, so a tab that keeps the header keeps it; before Approve
    // and the components.
    expect(view.querySelector(':scope > .head')!.contains(cuts)).toBe(true);
    expect(view.querySelector(':scope > .head .meta')!.compareDocumentPosition(cuts) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    for (const later of ['.approve-bar', '.node-changes']) {
      const at = view.querySelector(later);
      expect(at).not.toBeNull();
      expect(cuts.compareDocumentPosition(at!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    }
    for (const plain of [review({ branch: 'spec/plain', cuts: [] }), review({ branch: 'spec/plain', cuts: undefined })]) {
      const v = renderReview(plain, opts());
      expect(v.querySelector('.cuts')).toBeNull();
      expect([...v.querySelectorAll('h3')].map((h) => h.textContent)).not.toContain('Cuts');
    }
  });

  // req:web.views/spec-branch-review@bf6312ca
  it('says not approved and checked, and shows no page link without a page', () => {
    const head = renderReview(review({ page: null }), opts()).querySelector<HTMLElement>('.head')!;
    expect(head.textContent).toContain('not approved');
    expect([...head.querySelectorAll('.chip')].map((c) => c.textContent)).toContain('checked');
    expect(head.querySelector('a.page')).toBeNull();
  });
  // req:web.views/spec-branch-review@bf6312ca
  it('empty: no branch says No spec branches.; a review changing no component names its branch, base and what it resolves, and says No component changes.; one changing a and b names both', () => {
    expect(renderPrList([], link).querySelector('.empty')?.textContent).toBe('No spec branches.');
    expect(renderPrList([], link).querySelector('.prl')).toBeNull();
    const base = review();
    const x = renderReview(review({ branch: 'spec/x', base: 'main', nodes: [], change: { ...base.change!, resolves: ['crash'] } }), opts());
    const xHead = x.querySelector<HTMLElement>(':scope > .head')!;
    expect(xHead.querySelector('h1')?.textContent).toBe('spec/x');
    expect(xHead.querySelector('.meta')!.textContent).toContain('resolves crash');
    expect([...xHead.querySelectorAll('.meta code')].map((c) => c.textContent)).toEqual(['crash', 'spec/x → main']);
    expect(xHead.querySelectorAll('.chip.node')).toHaveLength(0);
    const both = renderReview(review({ change: { ...base.change!, resolves: ['crash', 'leak'] } }), opts());
    expect(both.querySelector(':scope > .head .meta')!.textContent).toContain('resolves crash, leak');
    expect(x.querySelector('.doc .note')?.textContent).toBe('No component changes.');
    expect(x.querySelector('.node-changes')).toBeNull();
    const node = base.nodes[0];
    const y = renderReview(review({ branch: 'spec/y', nodes: [{ ...node, id: 'a' }, { ...node, id: 'b' }] }), opts());
    expect([...y.querySelectorAll(':scope > .head .meta .chip.node')].map((c) => c.textContent)).toEqual(['a', 'b']);
    expect(y.querySelector('.doc')!.textContent).not.toContain('No component changes.');
  });
});

describe('Approve intent', () => {
  // req:web.views/approve-button@e2f53fdc
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

  // req:web.views/approve-button@e2f53fdc
  it('disabled: without a fingerprint, once an approval shows, and offline; the approved one reads Approved', () => {
    const approve = (r: BranchReview, o: ReturnType<typeof opts>) => renderReview(r, o).querySelector<HTMLButtonElement>('button.approve')!;
    const noFingerprint = approve(review({ fingerprint: null }), opts());
    const approved = approve(review({ approvals: [{ writer: 'a/web', at: '2026-10-27T10:00:00Z' }] }), opts());
    const offline = approve(review(), opts({ offline: true }));
    expect([noFingerprint, approved, offline].map((b) => b.disabled)).toEqual([true, true, true]);
    expect(approved.textContent).toBe('Approved');
    expect(noFingerprint.textContent).toBe('Approve intent');
    expect(offline.textContent).toBe('Approve intent');
    expect(approve(review(), opts()).disabled).toBe(false);
  });
});

describe('what Approve may send', () => {
  // req:web.views/approve-button@e2f53fdc
  it('no fingerprint: Approve is disabled', () => {
    expect(cannotAct(review({ fingerprint: null }), false, false)).not.toBeNull();
    expect(renderReview(review({ fingerprint: null }), opts()).querySelector<HTMLButtonElement>('button.approve')!.disabled).toBe(true);
  });

  // req:web.views/approve-button@e2f53fdc
  it('an approval on screen comes from the review, not from a label', () => {
    expect(cannotAct(review(), false, false)).toBeNull();
    expect(cannotAct(review({ approvals: [{ writer: 'a/web', at: 'x' }] }), false, false)).toBe('Approved');
  });
});

describe('no review list', () => {
  // req:web.views/spec-branch-review@bf6312ca
  it('the PR list shows the spec branches only: no To review list and no Mark reviewed', () => {
    const list = renderPrList(branches, link);
    expect(list.querySelector('.to-review')).toBeNull();
    expect(list.textContent).not.toContain('To review');
    expect([...list.querySelectorAll('button')].map((b) => b.textContent)).not.toContain('Mark reviewed');
    expect([...list.querySelectorAll<HTMLElement>('.prl')].map((r) => r.dataset.branch)).toEqual(branches.map((b) => b.branch));
  });
});

describe('signed repositories', () => {
  const approvalMarks = (view: HTMLElement) => [...view.querySelectorAll('.head .sig')].map((c) => c.textContent);

  // req:web.views/signed-approvals@bdf1ce5c
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
    const head = view.querySelector<HTMLElement>('.head')!;
    expect(head.textContent).toContain('clunny/web');
    expect(approvalMarks(view)).toEqual(["unsigned, doesn't count"]);
    expect(head.textContent).toContain('no signed approval');
    // Approvals are drawn neutral, not green, in a signed repository.
    expect([...head.querySelectorAll('.chip.ok')].some((c) => c.textContent?.includes('clunny/web'))).toBe(false);
    button.click();
    expect(o.onAct).not.toHaveBeenCalled();
  });

  // req:web.views/signed-approvals@bdf1ce5c
  it('signed approval: marked signed, and no "no signed approval"', () => {
    const view = renderReview(review({ branch: 'spec/x', signed: true, approvals: [{ writer: 'dana/9f9f9f9f', at: '2026-10-27T10:00:00Z', signed: true }] }), opts());
    expect(approvalMarks(view)).toEqual(['signed']);
    expect(view.querySelector('.head')?.textContent).not.toContain('no signed approval');
    expect(view.querySelector('button.approve')?.textContent).toBe('Approve intent');
  });

  // req:web.views/signed-approvals@bdf1ce5c
  it('signed with no approvals says not approved, and the note shows offline too', () => {
    const view = renderReview(review({ branch: 'spec/x', signed: true }), opts({ offline: true }));
    expect(view.querySelector('.head')?.textContent).toContain('not approved');
    expect(view.querySelector('.head')?.textContent).not.toContain('no signed approval');
    expect(view.querySelector('.signed-note code')?.textContent).toBe('clm approve spec/x');
  });

  it('copy puts the command on the clipboard', () => {
    const writeText = vi.fn(() => Promise.resolve());
    const kept = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    try {
      const view = renderReview(review({ branch: 'spec/x', signed: true }), opts());
      view.querySelector<HTMLButtonElement>('.signed-note button.copy')!.click();
      expect(writeText).toHaveBeenCalledWith('clm approve spec/x');
    } finally {
      if (kept) Object.defineProperty(navigator, 'clipboard', kept);
      else delete (navigator as { clipboard?: unknown }).clipboard;
    }
  });

  // req:web.views/signed-approvals@bdf1ce5c
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

const settle = () => new Promise((go) => setTimeout(go));
const typeIn = (form: HTMLElement, text: string) => {
  const t = form.querySelector('textarea')!;
  t.value = text;
  t.dispatchEvent(new Event('input'));
};

describe('Request changes', () => {
  const held = (over = {}) => ({ id: 'q1', writer: 'alice/web', at: '2026-10-27T12:00:00Z', fingerprint: 'f1', text: 'Split it', current: true, held: true, ...over });
  const button = (view: HTMLElement) => view.querySelector<HTMLButtonElement>('.approve-bar button.request')!;
  beforeEach(() => drafts.clear());

  // req:web.views/request-changes@db57a569
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
    expect(view.querySelector('.approve-bar .fbform')).toBeNull();

    const after = renderReview(review({ branch: 'spec/x', signed: true, change_requests: [held()] }), opts());
    const h = after.querySelector<HTMLElement>('.head .changes-held')!;
    expect(h.textContent).toContain('Changes requested by alice');
    expect(h.textContent).not.toContain('alice/web');
    expect(h.textContent).toContain('2026-10-27 12:00 UTC');
    expect(h.textContent).toContain('Split it');
    expect(h.textContent).toContain("won't land until the author pushes a change or alice approves it");
    // A held request is the header's, not a comment under the intent.
    expect(after.querySelector('.intent .fb')).toBeNull();
    // A request that no longer holds the branch isn't the header's.
    const settled = renderReview(review({ change_requests: [held({ held: false })] }), opts());
    expect(settled.querySelector('.head .changes-held')).toBeNull();
  });

  // req:web.views/request-changes@db57a569
  it('disabled: without a fingerprint and offline', () => {
    expect(button(renderReview(review({ fingerprint: null }), opts())).disabled).toBe(true);
    expect(button(renderReview(review(), opts({ offline: true }))).disabled).toBe(true);
  });

  // req:web.views/request-changes@db57a569
  it('enabled in a signed repository, after an approval and after an earlier request', () => {
    for (const r of [
      review({ signed: true }),
      review({ approvals: [{ writer: 'a/web', at: '2026-10-27T10:00:00Z' }] }),
      review({ change_requests: [held()] }),
    ]) {
      expect(button(renderReview(r, opts())).disabled).toBe(false);
    }
  });

  // req:web.views/request-changes@db57a569
  it("a refused or failed request shows why under the form, keeping the form and its text; an empty one sends nothing", async () => {
    for (const [onRequest, why] of [
      [vi.fn(() => Promise.resolve('Not sent: the Worker refused the text.')), 'Not sent: the Worker refused the text.'],
      [vi.fn(() => Promise.reject(new Error('down'))), 'Not sent: offline.'],
    ] as const) {
      drafts.clear();
      const view = renderReview(review(), opts({ onRequest }));
      button(view).click();
      const form = view.querySelector<HTMLElement>('.approve-bar .fbform')!;
      typeIn(form, ' Split it ');
      form.querySelector<HTMLButtonElement>('button.send')!.click();
      await settle();
      expect(onRequest).toHaveBeenCalledWith('Split it');
      expect(view.querySelector('.approve-bar .fbform .err')?.textContent).toBe(why);
      expect(view.querySelector<HTMLTextAreaElement>('.approve-bar .fbform textarea')?.value).toBe(' Split it ');
      // Kept for the redraw a refresh makes, too.
      expect(renderReview(review(), opts()).querySelector<HTMLTextAreaElement>('.approve-bar .fbform textarea')?.value).toBe(' Split it ');
    }
    drafts.clear();
    const onRequest = vi.fn(() => Promise.resolve(null));
    const view = renderReview(review(), opts({ onRequest }));
    button(view).click();
    typeIn(view.querySelector<HTMLElement>('.approve-bar .fbform')!, '   ');
    view.querySelector<HTMLButtonElement>('.approve-bar .fbform button.send')!.click();
    await settle();
    expect(onRequest).not.toHaveBeenCalled();
    expect(view.querySelector('.approve-bar .fbform .err')?.textContent).toBe('Write something first.');
    // Kept until cancelled.
    view.querySelector<HTMLButtonElement>('.approve-bar .fbform button.cancel')!.click();
    expect(renderReview(review(), opts()).querySelector('.approve-bar .fbform')).toBeNull();
  });
});

describe('comments', () => {
  const comment = (over = {}) => ({ id: 'c1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', fingerprint: 'f1', target: null, text: 'Looks right', current: true, ...over });
  const open = (root: HTMLElement, at: string) => {
    root.querySelector<HTMLButtonElement>(`${at} button.comment`)!.click();
    return root.querySelector<HTMLElement>(`${at} .fbform`)!;
  };
  beforeEach(() => drafts.clear());

  // req:web.views/comment-on-review@85d2f160
  it('comment on a requirement: one call with the target and trimmed text, and the card shows the comment by alice', async () => {
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

  // req:web.views/comment-on-review@85d2f160
  it('the intent carries Comment too, sending a null target', () => {
    const onComment = vi.fn(() => Promise.resolve(null));
    const view = renderReview(review(), opts({ onComment }));
    const form = open(view, '.intent');
    typeIn(form, 'Why now?');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    expect(onComment).toHaveBeenCalledWith(null, 'Why now?');
  });

  // req:web.views/comment-on-review@85d2f160
  it('sends nothing for an empty text, and says so', async () => {
    const onComment = vi.fn(() => Promise.resolve(null));
    const view = renderReview(review(), opts({ onComment }));
    const form = open(view, '.intent');
    typeIn(form, '   ');
    form.querySelector<HTMLButtonElement>('button.send')!.click();
    await settle();
    expect(onComment).not.toHaveBeenCalled();
    expect(form.querySelector('.err')?.textContent).toBe('Write something first.');
  });

  // req:web.views/comment-on-review@85d2f160
  it('earlier version: the comment under the intent marked on an earlier version, the request under it marked since approved', () => {
    const r = review({
      fingerprint: 'f2',
      comments: [comment({ fingerprint: 'f1', current: false })],
      change_requests: [
        { id: 'q1', writer: 'bob/web', at: '2026-10-27T11:00:00Z', fingerprint: 'f2', text: 'Split it', current: true, held: false },
        { id: 'q0', writer: 'bob/web', at: '2026-10-27T09:00:00Z', fingerprint: 'f1', text: 'Older', current: false, held: false },
      ],
    });
    const items = [...renderReview(r, opts()).querySelectorAll<HTMLElement>('.intent .fb')];
    expect(items.map((i) => i.textContent?.includes('on an earlier version'))).toEqual([true, false, true]);
    expect(items[2].textContent).not.toContain('since approved');
    expect(items[1].textContent).toContain('since approved');
    expect(items[1].textContent).toContain('bob');
    expect(items[1].textContent).toContain('Split it');
  });

  // req:web.views/comment-on-review@85d2f160
  it('a comment on a requirement the review shows no card for is drawn under the intent', () => {
    const view = renderReview(review({ comments: [comment({ target: 'n/gone' })] }), opts());
    expect(view.querySelector('.intent .fb')?.textContent).toContain('Looks right');
    expect(view.querySelector('.intent .fb')?.textContent).toContain('on n/gone');
    expect(view.querySelectorAll('.req .fb')).toHaveLength(0);
  });

  // req:web.views/comment-on-review@85d2f160
  it("drafts are kept per review scope, so another repository's branch of the same name has its own", () => {
    const x = review({ branch: 'spec/x' });
    const a = opts({ scope: '/api/repos/o/a/branches/spec/x' });
    const first = renderReview(x, a);
    typeIn(open(first, '.intent'), 'Name it');
    first.querySelector<HTMLButtonElement>('.approve-bar button.request')!.click();
    typeIn(first.querySelector<HTMLElement>('.approve-bar .fbform')!, 'Split');

    const again = renderReview(x, a);
    expect(again.querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('Name it');
    expect(again.querySelector<HTMLTextAreaElement>('.approve-bar .fbform textarea')?.value).toBe('Split');

    const other = renderReview(x, opts({ scope: '/api/repos/o/b/branches/spec/x' }));
    expect(other.querySelector('.fbform')).toBeNull();
  });

  // req:web.views/comment-on-review@85d2f160
  it('a review without comments or change_requests reads as having none', () => {
    const r = review();
    delete r.comments;
    delete r.change_requests;
    expect(renderReview(r, opts()).querySelectorAll('.fb')).toHaveLength(0);
  });

  // req:web.views/comment-on-review@85d2f160
  it('a refusal shows its reason under the form and keeps it', async () => {
    for (const [onComment, why] of [
      [vi.fn(() => Promise.resolve('Not sent: the Worker refused the text.')), 'Not sent: the Worker refused the text.'],
      [vi.fn(() => Promise.reject(new Error('down'))), 'Not sent: offline.'],
    ] as const) {
      drafts.clear();
      const view = renderReview(review(), opts({ onComment }));
      const form = open(view, '.intent');
      typeIn(form, 'x');
      form.querySelector<HTMLButtonElement>('button.send')!.click();
      await settle();
      expect(onComment).toHaveBeenCalledTimes(1);
      expect(view.querySelector('.intent .fbform .err')?.textContent).toBe(why);
      expect(view.querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('x');
      expect(renderReview(review(), opts()).querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('x');
    }
  });

  // req:web.views/comment-on-review@85d2f160
  it('what was typed survives a redraw', () => {
    const first = renderReview(review(), opts());
    typeIn(open(first, '.intent'), 'half a thought');
    const again = renderReview(review(), opts());
    expect(again.querySelector<HTMLTextAreaElement>('.intent .fbform textarea')?.value).toBe('half a thought');
    // Until it is cancelled.
    again.querySelector<HTMLButtonElement>('.intent .fbform button.cancel')!.click();
    expect(renderReview(review(), opts()).querySelector('.intent .fbform')).toBeNull();
  });

  // req:web.views/comment-on-review@85d2f160
  it('disabled offline and without a fingerprint, on the intent and every card', () => {
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
