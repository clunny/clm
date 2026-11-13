// The answer buttons of a backlog item (web.views/answer-buttons): one per choice of a
// `question`, Done on a `human-task`, none on any other item. Home and the item's page draw
// them alike.

import { div, el } from './dom';
import type { Answer, BacklogView } from './types';

export interface Answering {
  /** The implementation state's open answers; empty until it arrives. */
  answers: Answer[];
  offline: boolean;
  /** Posts `answer` for item `slug` and asks for the implementation state again: null once the
   * Worker took it, else why not, shown under the buttons. */
  onAnswer: (slug: string, answer: string) => Promise<string | null>;
}

/** The answers the item offers, by the value each sends and the label it shows; null when it
 * offers none. */
export function offered(b: BacklogView): { value: string; label: string }[] | null {
  if (b.kind === 'human-task') return [{ value: 'done', label: 'Done' }];
  if (b.kind === 'question' && (b.choices ?? []).length > 0) return (b.choices ?? []).map((c) => ({ value: c, label: c }));
  return null;
}

/** The buttons of `b`, or nothing when it offers none. A tap posts once and none runs while
 * it is in flight; the button of the person's own answer is drawn chosen. */
export function answerButtons(b: BacklogView, a: Answering): HTMLElement | '' {
  const choices = offered(b);
  if (!choices) return '';
  const mine = new Set(a.answers.filter((x) => x.item === b.slug && x.mine).map((x) => x.answer));
  const err = div('err');
  const buttons = choices.map((c) => {
    const chosen = mine.has(c.value);
    return el(
      'button',
      {
        type: 'button',
        class: chosen ? 'btn pri answer chosen' : 'btn answer',
        'data-answer': c.value,
        'aria-pressed': chosen ? 'true' : 'false',
        disabled: a.offline,
        title: a.offline ? 'Offline' : c.label,
      },
      c.label,
    );
  });
  let busy = false;
  for (const [i, btn] of buttons.entries()) {
    btn.addEventListener('click', async () => {
      if (busy || btn.disabled) return;
      busy = true;
      err.textContent = '';
      const why = await a.onAnswer(b.slug, choices[i].value).catch(() => 'Not sent: offline.');
      busy = false;
      if (why !== null) err.textContent = why;
    });
  }
  return div('answers', div('abtns', ...buttons), err);
}
