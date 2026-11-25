import { describe, expect, it, vi } from 'vitest';
import { answerButtons, type Answering } from './answers';
import { renderBacklogItem } from './backlog';
import { snapshot } from './fixtures';
import { renderHome } from './home';
import type { Answer, BacklogView } from './types';

const item = (slug: string, kind: string, over: Partial<BacklogView> = {}): BacklogView => ({ slug, kind, title: `Title ${slug}`, nodes: ['a'], origin: null, blocks: [], ...over });
const alice = (over: Partial<Answer> = {}): Answer => ({ id: 'a1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', item: 'q', answer: 'yes', mine: true, ...over });
const answering = (over: Partial<Answering> = {}): Answering => ({ answers: [], offline: false, onAnswer: async () => null, ...over });
const labels = (e: HTMLElement | '') => (e ? [...e.querySelectorAll<HTMLButtonElement>('button')].map((b) => b.textContent) : []);
const tick = () => new Promise((r) => setTimeout(r, 0));

describe('answer buttons', () => {
  // req:web.views/answer-buttons@1248b040
  it('a question offers one button per choice in order, a human-task Done, and no other item any', () => {
    expect(labels(answerButtons(item('q', 'question', { choices: ['yes', 'no', 'later'] }), answering()))).toEqual(['yes', 'no', 'later']);
    expect(labels(answerButtons(item('h', 'human-task'), answering()))).toEqual(['Done']);
    expect(answerButtons(item('r', 'question'), answering())).toBe('');
    expect(answerButtons(item('i', 'idea', { choices: ['yes'] }), answering())).toBe('');
    expect(answerButtons(item('b', 'bug'), answering())).toBe('');
  });

  // req:web.views/answer-buttons@1248b040
  it('answered: two quick taps post once, and the person\'s own answer is drawn chosen', async () => {
    const q = item('q', 'question', { choices: ['yes', 'no'] });
    let release = () => {};
    const onAnswer = vi.fn(() => new Promise<string | null>((r) => (release = () => r(null))));
    const view = answerButtons(q, answering({ onAnswer })) as HTMLElement;
    const yes = view.querySelector<HTMLButtonElement>('button[data-answer=yes]')!;
    yes.click();
    yes.click();
    view.querySelector<HTMLButtonElement>('button[data-answer=no]')!.click();
    release();
    await tick();
    expect(onAnswer.mock.calls).toEqual([['q', 'yes']]);
    // Once the implementation state shows the answers, the view is drawn with the person's own
    // chosen, and none for another person's.
    const state = [alice(), alice({ id: 'a2', writer: 'bob/web', answer: 'no', mine: false })];
    const after = answerButtons(q, answering({ answers: state })) as HTMLElement;
    expect(labels(after)).toEqual(['yes', 'no']);
    expect([...after.querySelectorAll<HTMLButtonElement>('button.chosen')].map((b) => b.dataset.answer)).toEqual(['yes']);
    expect(after.querySelector('button[data-answer=no]')!.getAttribute('aria-pressed')).toBe('false');
    // An answer to another item is not this item's.
    const elsewhere = answerButtons(q, answering({ answers: [alice({ item: 'r' })] })) as HTMLElement;
    expect(elsewhere.querySelector('.chosen')).toBeNull();
    // Another tap sends again once the first is done.
    yes.click();
    expect(onAnswer).toHaveBeenCalledTimes(2);
  });

  // req:web.views/answer-buttons@1248b040
  it('done: Done sends done', async () => {
    const onAnswer = vi.fn(async () => null);
    const page = renderBacklogItem(snapshot({ backlog: [item('h', 'human-task')] }), 'h', null, '#/o/a/backlog', answering({ onAnswer }));
    page.querySelector<HTMLButtonElement>('button.answer')!.click();
    await tick();
    expect(onAnswer.mock.calls).toEqual([['h', 'done']]);
  });

  // req:web.views/answer-buttons@1248b040
  it('a refusal shows why under the buttons and keeps them enabled', async () => {
    const view = answerButtons(item('q', 'question', { choices: ['yes', 'no'] }), answering({ onAnswer: async () => 'Not sent (422).' })) as HTMLElement;
    view.querySelector<HTMLButtonElement>('button[data-answer=yes]')!.click();
    await tick();
    expect(view.querySelector('.err')!.textContent).toBe('Not sent (422).');
    expect([...view.querySelectorAll<HTMLButtonElement>('button')].some((b) => b.disabled)).toBe(false);
    const thrown = answerButtons(item('q', 'question', { choices: ['yes'] }), answering({ onAnswer: async () => Promise.reject(new Error('x')) })) as HTMLElement;
    thrown.querySelector<HTMLButtonElement>('button')!.click();
    await tick();
    expect(thrown.querySelector('.err')!.textContent).toBe('Not sent: offline.');
  });

  // req:web.views/answer-buttons@1248b040
  it('offline: the buttons are disabled and send nothing', () => {
    const onAnswer = vi.fn(async () => null);
    const view = answerButtons(item('q', 'question', { choices: ['yes', 'no'] }), answering({ offline: true, onAnswer })) as HTMLElement;
    const all = [...view.querySelectorAll<HTMLButtonElement>('button')];
    expect(all.every((b) => b.disabled)).toBe(true);
    all[0].click();
    expect(onAnswer).not.toHaveBeenCalled();
    // So are they on the item's page.
    const snap = snapshot({ backlog: [item('q', 'question', { choices: ['yes', 'no'] })] });
    const page = renderBacklogItem(snap, 'q', null, '#/o/a/backlog', answering({ offline: true, onAnswer }));
    const onPage = [...page.querySelectorAll<HTMLButtonElement>('button.answer')];
    expect(onPage).toHaveLength(2);
    expect(onPage.every((b) => b.disabled)).toBe(true);
  });

  // req:web.views/home-waiting-on-you@76d80a78
  it('Home draws the buttons under each Answer entry', () => {
    const snap = snapshot({ backlog: [item('q', 'question', { choices: ['yes', 'no'] }), item('h', 'human-task')] });
    const home = renderHome({
      snap,
      reviews: [],
      caps: null,
      arch: null,
      reviewLink: (b) => b,
      itemLink: (s) => `#/o/a/backlog/${s}`,
      capLink: (s) => s,
      archLink: (s) => s,
      answering: answering(),
    });
    const rows = [...home.querySelectorAll<HTMLElement>('.hsec[data-section=answer] .hent-row')];
    expect(rows.map((r) => labels(r))).toEqual([['yes', 'no'], ['Done']]);
  });
});
