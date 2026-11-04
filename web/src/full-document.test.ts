// req:web/full-document@fcfa5b34
import { afterEach, describe, expect, it, vi } from 'vitest';
import { blocks, renderFullDocument } from './full-document';
import { SPEC_AFTER, SPEC_BEFORE } from '../test/fixtures';

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

  it('shows the whole file as it reads at the head, unchanged text dimmed, changes marked', () => {
    const { view } = open();
    const text = view.el.querySelector('.fdoc')!.textContent!;
    expect(text).toContain('The contract.');
    expect(view.el.querySelector('.blk.ctx')).not.toBeNull();
    expect(view.el.querySelector('.chg ins')?.textContent).toContain('now changed');
  });

  it('folds long unchanged runs, and a fold opens', () => {
    const { view } = open();
    const fold = view.el.querySelector<HTMLButtonElement>('.fold')!;
    expect(fold.textContent).toMatch(/unchanged/);
    fold.click();
    expect(view.el.querySelector('.fold')).toBeNull();
    expect(view.el.querySelector('.fdoc')!.textContent).toContain('Fifth.');
  });

  it('j and k step through the changes', () => {
    const { view } = open();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'j' }));
    expect(view.current()).toBe(1);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k' }));
    expect(view.current()).toBe(0);
    view.close();
  });

  it('after and before modes show one side, still numbered', () => {
    expect(open('after').view.el.querySelector('.chg del')).toBeNull();
    const before = open('before').view;
    expect(before.el.querySelector('.chg ins')).toBeNull();
    expect(before.el.querySelector('.chg')?.textContent).not.toContain('now changed');
  });

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

  it('a file added whole is one change', () => {
    const changes = new Set(blocks(null, 'One.\n\nTwo.').map((b) => b.change));
    expect([...changes]).toEqual([0]);
  });
});
