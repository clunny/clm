// req:web.views/backlog-item-page@c5058fba
import { describe, expect, it } from 'vitest';
import type { Answering } from './answers';
import { renderBacklogItem } from './backlog';
import { snapshot } from './fixtures';
import type { Taken } from './types';

const q = () =>
  snapshot({
    backlog: [{ slug: 'q', kind: 'question', title: 'Ship it?', nodes: ['a'], origin: 'c', blocks: ['a/x'], priority: 'P2', body: 'Should we **ship**?', choices: ['yes', 'no'] }],
  });
const answering: Answering = {
  answers: [
    { id: 'a1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', item: 'q', answer: 'yes', mine: true },
    { id: 'a2', writer: 'bob/web', at: '2026-10-27T11:00:00Z', item: 'other', answer: 'no', mine: false },
  ],
  offline: false,
  onAnswer: async () => null,
};

describe('a backlog item page', () => {
  it('an item in full: title, kind, priority, components, origin, blocks, the answer by its person, buttons, and the body as Markdown', () => {
    const page = renderBacklogItem(q(), 'q', null, '#/o/a/backlog', answering);
    expect(page.querySelector('h1')!.textContent).toBe('Ship it?');
    const text = page.textContent!;
    for (const part of ['question', 'P2', 'a/x', 'Origin: c', 'yes by alice']) expect(text).toContain(part);
    expect([...page.querySelectorAll('.meta .chip')].map((c) => c.textContent)).toEqual(['question', 'P2', 'a']);
    expect(text).not.toContain('bob');
    expect(page.querySelector('.md strong')!.textContent).toBe('ship');
    expect(page.querySelectorAll('button.answer')).toHaveLength(2);
    // The body comes after the answer buttons.
    const order = [...page.children].map((c) => c.className.split(' ')[0]);
    expect(order.indexOf('md')).toBeGreaterThan(order.indexOf('answers'));
  });

  it('shows what holds the item', () => {
    const taken: Taken = { changes: [{ id: 'x', state: 'proposed', items: ['q'], nodes: [] }], claims: [] };
    expect(renderBacklogItem(q(), 'q', taken, '#/o/a/backlog').querySelector('.taken')!.textContent).toBe('proposed in x');
  });

  it('sanitises the body', () => {
    const snap = q();
    snap.backlog[0].body = 'hi <img src=x onerror=alert(1)>';
    const page = renderBacklogItem(snap, 'q', null, '#/o/a/backlog');
    expect(page.innerHTML).not.toContain('onerror');
  });

  it('an item with no priority or answers shows none, and no buttons without an answerer', () => {
    const snap = snapshot();
    const page = renderBacklogItem(snap, 'crash', null, '#/o/a/backlog');
    expect(page.querySelector('.prio')).toBeNull();
    expect(page.querySelector('.given')).toBeNull();
    expect(page.querySelector('.answers')).toBeNull();
  });

  it('gone: says the item is not in the backlog and links to the Backlog', () => {
    const page = renderBacklogItem(snapshot(), 'gone', null, '#/o/a/backlog', answering);
    expect(page.textContent).toContain("isn't in the backlog");
    expect(page.querySelector('a:not(.back)')!.getAttribute('href')).toBe('#/o/a/backlog');
  });
});
