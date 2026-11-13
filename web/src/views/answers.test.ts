// req:web.views/answer-buttons@c08a8031
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
  it('a question offers one button per choice in order, a human-task Done, and no other item any', () => {
    expect(labels(answerButtons(item('q', 'question', { choices: ['yes', 'no', 'later'] }), answering()))).toEqual(['yes', 'no', 'later']);
    expect(labels(answerButtons(item('h', 'human-task'), answering()))).toEqual(['Done']);
    expect(answerButtons(item('r', 'question'), answering())).toBe('');
    expect(answerButtons(item('i', 'idea', { choices: ['yes'] }), answering())).toBe('');
    expect(answerButtons(item('b', 'bug'), answering())).toBe('');
  });

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
    // Once the implementation state shows the answer, the view is drawn with it chosen.
    const after = answerButtons(q, answering({ answers: [alice()] })) as HTMLElement;
    expect(after.querySelector('button[data-answer=yes]')!.classList.contains('chosen')).toBe(true);
    expect(after.querySelector('button[data-answer=no]')!.classList.contains('chosen')).toBe(false);
    // Another person's answer isn't the person's own.
    const other = answerButtons(q, answering({ answers: [alice({ mine: false })] })) as HTMLElement;
    expect(other.querySelector('.chosen')).toBeNull();
    // Another tap sends again once the first is done.
    yes.click();
    expect(onAnswer).toHaveBeenCalledTimes(2);
  });

  it('done: Done sends done', async () => {
    const onAnswer = vi.fn(async () => null);
    const page = renderBacklogItem(snapshot(), 'setup', null, '#/o/a/backlog', answering({ onAnswer }));
    page.querySelector<HTMLButtonElement>('button.answer')!.click();
    await tick();
    expect(onAnswer.mock.calls).toEqual([['setup', 'done']]);
  });

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

  it('offline: the buttons are disabled and send nothing', () => {
    const onAnswer = vi.fn(async () => null);
    const view = answerButtons(item('q', 'question', { choices: ['yes', 'no'] }), answering({ offline: true, onAnswer })) as HTMLElement;
    const all = [...view.querySelectorAll<HTMLButtonElement>('button')];
    expect(all.every((b) => b.disabled)).toBe(true);
    all[0].click();
    expect(onAnswer).not.toHaveBeenCalled();
  });

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
