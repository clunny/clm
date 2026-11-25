import { afterEach, describe, expect, it, vi } from 'vitest';
import { blocks, renderFullDocument } from './full-document';
import { SPEC_AFTER, SPEC_BEFORE } from './fixtures';

const doc = { path: 'specs/n/SPEC.md', before: SPEC_BEFORE, after: SPEC_AFTER };

afterEach(() => vi.restoreAllMocks());

function open(mode: 'inline' | 'after' | 'before' = 'inline') {
  const scrolled: Element[] = [];
  // jsdom doesn't lay out, so it has no scrollIntoView to spy on.
  Element.prototype.scrollIntoView ??= () => {};
  vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(function (this: Element) {
    scrolled.push(this);
  });
  const view = renderFullDocument(doc, mode);
  document.body.replaceChildren(view.el);
  view.go(0);
  return { view, scrolled };
}

describe('full document', () => {
  // req:web.views/full-document@7507d96d
  it('open at first change: Design is change 1 of 2, and next brings the requirement', () => {
    const { view, scrolled } = open();
    expect(view.total).toBe(2);
    expect(view.current()).toBe(0);
    const focus = view.el.querySelector<HTMLElement>('.chg.focus')!;
    expect(focus.textContent).toContain('first paragraph');
    expect(view.el.querySelector('.pos')?.textContent).toContain('1 of 2');
    expect(scrolled.at(-1)).toBe(focus);

    view.next();
    const second = view.el.querySelector<HTMLElement>('.chg.focus')!;
    expect(second.textContent).toContain('s twice');
    expect(view.el.querySelector('.pos')?.textContent).toContain('2 of 2');
    expect(scrolled.at(-1)).toBe(second);

    view.prev();
    expect(view.current()).toBe(0);
  });

  // req:web.views/full-document@7507d96d
  it('shows the whole file as it reads at the head, unchanged text dimmed, changes marked', () => {
    const { view } = open();
    const text = view.el.querySelector('.fdoc')!.textContent!;
    expect(text).toContain('The contract.');
    expect(view.el.querySelector('.blk.ctx')).not.toBeNull();
    expect(view.el.querySelector('.chg ins')?.textContent).toContain('now changed');
  });

  // req:web.views/full-document@7507d96d
  it('folds long unchanged runs, and a fold opens', () => {
    const { view } = open();
    const fold = view.el.querySelector<HTMLButtonElement>('.fold')!;
    expect(fold.textContent).toMatch(/unchanged/);
    fold.click();
    expect(view.el.querySelector('.fold')).toBeNull();
    expect(view.el.querySelector('.fdoc')!.textContent).toContain('Fifth.');
  });

  // req:web.views/full-document@7507d96d
  it('j and k step through the changes', () => {
    const { view } = open();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'j' }));
    expect(view.current()).toBe(1);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k' }));
    expect(view.current()).toBe(0);
    view.close();
  });

  // req:web.views/full-document@7507d96d
  it('after and before modes show one side, still numbered', () => {
    expect(open('after').view.el.querySelector('.chg del')).toBeNull();
    const before = open('before').view;
    expect(before.el.querySelector('.chg ins')).toBeNull();
    expect(before.el.querySelector('.chg')?.textContent).not.toContain('now changed');
  });

  // req:web.views/full-document@7507d96d
  it('the outline marks where the changes are', () => {
    const { view } = open();
    const entries = [...view.el.querySelectorAll('.toc .t')].map((t) => [t.textContent, !!t.querySelector('.m')]);
    expect(entries).toEqual([
      ['N', false],
      ['Contract', false],
      ['Design', true],
      ['Requirements', false],
      ['r', false],
      ['s', true],
    ]);
  });

  // req:web.views/full-document@7507d96d
  it('a file added whole is one change', () => {
    const changes = new Set(blocks(null, 'One.\n\nTwo.').map((b) => b.change));
    expect([...changes]).toEqual([0]);
  });

  // req:web.views/full-document@7507d96d
  it('stepping and folding: keys stop at the ends and not in a field, the fold opens, the switch keeps the numbers, the outline goes to a change, a redraw keeps its place; an added file and an unchanged one', () => {
    Element.prototype.scrollIntoView ??= () => {};
    vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(() => {});
    const same = ['P1.', 'P2.', 'P3.', 'P4.', 'P5.', 'P6.'].join('\n\n');
    const file = (one: string, two: string) => `## Design\n\n${one}\n\n${same}\n\n### Requirement: r\n\n${two}\n`;
    const twoChanges = { path: 'specs/n/SPEC.md', before: file('One old.', 'Two old.'), after: file('One new.', 'Two new.') };
    const view = renderFullDocument(twoChanges);
    document.body.replaceChildren(view.el);
    const pos = () => view.el.querySelector('.pos')!.textContent;
    const key = (k: string, target: EventTarget = document) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }));
    expect(pos()).toBe('Change 1 of 2');
    for (let i = 0; i < 3; i++) key('j');
    expect(pos()).toBe('Change 2 of 2');
    key('k');
    expect(pos()).toBe('Change 1 of 2');
    const field = document.createElement('input');
    document.body.append(field);
    key('j', field);
    expect(pos()).toBe('Change 1 of 2');
    field.remove();
    const fold = view.el.querySelector<HTMLButtonElement>('.fold')!;
    expect(fold.textContent).toBe('⋯ 4 unchanged paragraphs');
    fold.click();
    expect(view.el.querySelector('.fold')).toBeNull();
    for (const p of ['P2.', 'P3.', 'P4.', 'P5.']) expect(view.el.querySelector('.fdoc')!.textContent).toContain(p);
    const seg = (label: string) => [...view.el.querySelectorAll<HTMLButtonElement>('.seg button')].find((b) => b.textContent === label)!;
    seg('After').click();
    expect(view.el.querySelector('.chg')!.textContent).toContain('One new.');
    expect(view.el.querySelector('.chg')!.textContent).not.toContain('One old.');
    expect([...view.el.querySelectorAll('.chg .why')].map((w) => w.textContent)).toEqual(['Change 1 · modified', 'Change 2 · modified']);
    seg('Before').click();
    expect(view.el.querySelector('.chg')!.textContent).toContain('One old.');
    expect(view.el.querySelector('.chg')!.textContent).not.toContain('One new.');
    expect([...view.el.querySelectorAll('.chg .why')].map((w) => w.textContent)).toEqual(['Change 1 · modified', 'Change 2 · modified']);
    const toc = [...view.el.querySelectorAll<HTMLElement>('.toc .t')];
    expect(toc.map((t) => [t.textContent, !!t.querySelector('.m')])).toEqual([
      ['Design', true],
      ['r', true],
    ]);
    toc[1].click();
    expect(pos()).toBe('Change 2 of 2');
    const kept = view.state();
    view.close();
    const again = renderFullDocument(twoChanges);
    again.restore(kept);
    expect(again.current()).toBe(1);
    again.close();

    const added = renderFullDocument({ path: 'docs/new.md', before: null, after: 'One.\n\nTwo.' });
    expect(added.el.querySelector('.pos')!.textContent).toBe('Change 1 of 1');
    added.close();
    const unchanged = renderFullDocument({ path: 'docs/same.md', before: 'One.', after: 'One.' });
    expect(unchanged.el.querySelector('.pos')!.textContent).toBe('No changes');
    unchanged.close();
  });
});
