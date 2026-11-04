// A changed file whole, as it reads at the PR's head: unchanged text dimmed, long unchanged
// runs folded, every change marked and numbered, an outline marking where they are, opened
// at the first change, with ↑/↓ and j/k stepping through them (web/full-document).

import { diffArrays } from 'diff';
import { wordMarks } from './diff';
import { div, el } from './dom';
import type { Document } from './types';

export type Mode = 'inline' | 'after' | 'before';

export interface Block {
  kind: 'same' | 'changed';
  before: string | null;
  after: string | null;
  /** The change this block belongs to, counting from 0; consecutive changed blocks share one. */
  change?: number;
}

/** Runs of more unchanged paragraphs than this fold, keeping the first and last. */
const FOLD_OVER = 3;

function paragraphs(text: string | null): string[] {
  if (!text) return [];
  return text
    .replace(/\r\n/g, '\n')
    .split(/\n[ \t]*\n/)
    .map((p) => p.replace(/^\n+|\s+$/g, ''))
    .filter((p) => p.length > 0);
}

export function blocks(before: string | null, after: string | null): Block[] {
  const out: Block[] = [];
  let removed: string[] = [];
  let added: string[] = [];
  const flush = () => {
    const n = Math.max(removed.length, added.length);
    for (let i = 0; i < n; i++) out.push({ kind: 'changed', before: removed[i] ?? null, after: added[i] ?? null });
    removed = [];
    added = [];
  };
  for (const part of diffArrays(paragraphs(before), paragraphs(after))) {
    if (part.removed) removed.push(...part.value);
    else if (part.added) added.push(...part.value);
    else {
      flush();
      for (const p of part.value) out.push({ kind: 'same', before: p, after: p });
    }
  }
  flush();
  let change = -1;
  out.forEach((b, i) => {
    if (b.kind !== 'changed') return;
    if (i === 0 || out[i - 1].kind !== 'changed') change++;
    b.change = change;
  });
  return out;
}

function isHeading(text: string | null): boolean {
  return !!text && /^#{1,6}\s/.test(text);
}

function headingText(text: string): string {
  return text.split('\n')[0].replace(/^#+\s*/, '').replace(/^Requirement:\s*/, '');
}

function what(b: Block): string {
  return b.before === null ? 'added' : b.after === null ? 'removed' : 'modified';
}

/** What a redraw keeps of the view: its switch and its current change (web/live-refresh). */
export interface FullState {
  mode: Mode;
  index: number;
  /** The current change's text, which finds it again when the changes move. */
  text: string;
}

export interface FullDocument {
  el: HTMLElement;
  total: number;
  current(): number;
  go(i: number): void;
  next(): void;
  prev(): void;
  state(): FullState;
  /** Goes to the change whose text is `s.text`, else the nearest by position. */
  restore(s: FullState): void;
  /** Stops listening for keys; call it when leaving the view. */
  close(): void;
}

export function renderFullDocument(doc: Document, mode: Mode = 'inline', back?: Node): FullDocument {
  const all = blocks(doc.before, doc.after);
  const total = all.reduce((n, b) => Math.max(n, (b.change ?? -1) + 1), 0);
  const unfolded = new Set<number>();
  let at = 0;
  let shown: Mode = mode;

  const root = div('full-doc card');
  const pos = el('span', { class: 'pos' });
  const changeEls = (): HTMLElement[] => [...root.querySelectorAll<HTMLElement>('.chg[data-change]')];

  function text(b: Block): Node[] {
    if (b.kind === 'same') return [document.createTextNode(b.after ?? '')];
    if (shown === 'after') return [document.createTextNode(b.after ?? '')];
    if (shown === 'before') return [document.createTextNode(b.before ?? '')];
    return wordMarks(b.before, b.after);
  }

  function blockEl(b: Block): HTMLElement {
    const t = shown === 'before' ? b.before : b.after ?? b.before;
    const cls = ['blk'];
    if (isHeading(t)) cls.push('hd');
    if (b.kind === 'same') cls.push('ctx');
    return el('div', { class: cls.join(' ') }, ...text(b));
  }

  function body(): HTMLElement {
    const out = div('fdoc');
    let i = 0;
    let run = 0;
    while (i < all.length) {
      const b = all[i];
      if (b.kind === 'changed') {
        const group = div('chg');
        group.dataset.change = String(b.change);
        const kinds = new Set<string>();
        while (i < all.length && all[i].change === b.change) {
          kinds.add(what(all[i]));
          group.append(blockEl(all[i]));
          i++;
        }
        group.prepend(div('why', `Change ${(b.change ?? 0) + 1} · ${[...kinds].join(', ')}`));
        out.append(group);
        continue;
      }
      // A run of unchanged blocks; long stretches of plain paragraphs fold.
      let j = i;
      while (j < all.length && all[j].kind === 'same' && !isHeading(all[j].after)) j++;
      const len = j - i;
      if (len > FOLD_OVER && !unfolded.has(run)) {
        const id = run;
        out.append(blockEl(all[i]));
        const fold = el('button', { type: 'button', class: 'fold' }, `⋯ ${len - 2} unchanged paragraphs`);
        fold.addEventListener('click', () => {
          unfolded.add(id);
          render();
        });
        out.append(fold, blockEl(all[j - 1]));
        i = j;
      } else if (len > 0) {
        for (; i < j; i++) out.append(blockEl(all[i]));
      } else {
        out.append(blockEl(all[i]));
        i++;
      }
      run++;
    }
    return out;
  }

  function outline(): HTMLElement {
    const toc = div('toc', div('h', 'Outline'));
    let entry: HTMLElement | null = null;
    for (const b of all) {
      const t = b.after ?? b.before;
      if (isHeading(t) && t) {
        const sub = /^#{1,6}\s+Requirement:/.test(t);
        entry = el('div', { class: sub ? 't sub' : 't' }, el('span', {}, headingText(t)));
        const target = b;
        entry.addEventListener('click', () => {
          const idx = all.indexOf(target);
          const next = all.slice(idx).find((x) => x.change !== undefined);
          if (next?.change !== undefined) go(next.change);
        });
        toc.append(entry);
      }
      if (b.change !== undefined && entry && !entry.querySelector('.m')) entry.append(el('span', { class: 'm' }));
    }
    return toc;
  }

  function bar(): HTMLElement {
    const seg = div('seg');
    for (const [m, label] of [
      ['inline', 'Changes inline'],
      ['after', 'After'],
      ['before', 'Before'],
    ] as [Mode, string][]) {
      const b = el('button', { type: 'button', class: m === shown ? 'on' : '' }, label);
      b.addEventListener('click', () => {
        shown = m;
        render();
        go(at);
      });
      seg.append(b);
    }
    const up = el('button', { type: 'button', class: 'ib', 'aria-label': 'Previous change' }, '↑');
    const down = el('button', { type: 'button', class: 'ib', 'aria-label': 'Next change' }, '↓');
    up.addEventListener('click', prev);
    down.addEventListener('click', next);
    return div('fbar', div('l', back ?? '', ' ', el('b', { class: 'mono' }, doc.path)), div('fnav', seg, up, pos, down));
  }

  function render() {
    root.replaceChildren(bar(), div('full', outline(), body()));
    mark();
  }

  function mark() {
    const els = changeEls();
    els.forEach((e, i) => e.classList.toggle('focus', i === at));
    pos.textContent = total ? `Change ${at + 1} of ${total}` : 'No changes';
  }

  function go(i: number) {
    if (!total) return mark();
    at = Math.max(0, Math.min(total - 1, i));
    mark();
    changeEls()[at]?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }
  function next() {
    go(at + 1);
  }
  function prev() {
    go(at - 1);
  }

  const onKey = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName)) return;
    if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault();
      next();
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault();
      prev();
    }
  };
  document.addEventListener('keydown', onKey);

  render();
  const changeText = (i: number) => changeEls()[i]?.textContent ?? '';
  return {
    el: root,
    total,
    current: () => at,
    go,
    next,
    prev,
    state: () => ({ mode: shown, index: at, text: changeText(at) }),
    restore(s) {
      if (s.mode !== shown) {
        shown = s.mode;
        render();
      }
      const same = changeEls().findIndex((e) => e.textContent === s.text);
      go(same >= 0 ? same : s.index);
    },
    close: () => document.removeEventListener('keydown', onKey),
  };
}
