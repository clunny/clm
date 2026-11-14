// req:web.views/home-waiting-on-you@46bb33db
import { describe, expect, it } from 'vitest';
import type { Answering } from './answers';
import { architecture, capabilities, constraint, cost, review, snapshot } from './fixtures';
import { homeCount, renderHome } from './home';
import type { BacklogView, Snapshot } from './types';

const item = (slug: string, kind: string, over: Partial<BacklogView> = {}): BacklogView => ({ slug, kind, title: `Title ${slug}`, nodes: ['a'], origin: null, blocks: [], ...over });

/** Repo `o/a`: `spec/x` needs review with no approval, `spec/y` needs review with one;
 * question `q` with choices, question `r` without, human-task `h`, and `c` flagged. */
function waiting(): Snapshot {
  return snapshot({ backlog: [item('q', 'question', { choices: ['yes', 'no'] }), item('r', 'question'), item('h', 'human-task'), item('i', 'idea')] });
}
const approval = { writer: 'alice/web', at: '2026-01-01T00:00:00Z', signed: false };
const reviews = () => [review({ branch: 'spec/x', approvals: [] }), review({ branch: 'spec/y', approvals: [approval] })];
const unflagged = () => capabilities({ report: { ...capabilities().report, capabilities: [cost('d', 1)] } });
const noneFlagged = () => architecture({ architecture: [constraint('g', 0)] });
const flagged = () => capabilities({ report: { ...capabilities().report, capabilities: [cost('c', 3, { flagged: true }), cost('d', 1)] } });
const answering: Answering = { answers: [], offline: false, onAnswer: async () => null };
const links = {
  answering,
  reviewLink: (b: string) => `#/o/a/prs/${encodeURIComponent(b)}`,
  itemLink: (s: string) => `#/o/a/backlog/${s}`,
  capLink: (s: string) => `#/o/a/capabilities/${s}`,
  archLink: (s: string) => `#/o/a/architecture/${s}`,
};
const section = (e: HTMLElement, name: string) => e.querySelector<HTMLElement>(`.hsec[data-section=${name}]`);
/** The labels of the answer buttons under each entry of the section, in order. */
const buttons = (s: HTMLElement | null) => [...(s?.querySelectorAll<HTMLElement>('.hent-row') ?? [])].map((r) => [...r.querySelectorAll('button.answer')].map((b) => b.textContent));
const entries = (s: HTMLElement | null) => [...(s?.querySelectorAll<HTMLAnchorElement>('a.hent') ?? [])].map((a) => [a.textContent, a.getAttribute('href')]);

describe('home: waiting on you', () => {
  it('what waits: Review, Answer, Flagged and Answer in a session, in that order, with the badge at 3', () => {
    const snap = waiting();
    const view = renderHome({ snap, reviews: reviews(), caps: flagged(), arch: noneFlagged(), ...links });
    expect([...view.querySelectorAll<HTMLElement>('.hsec')].map((s) => s.dataset.section)).toEqual(['review', 'answer', 'flagged', 'session']);
    expect(entries(section(view, 'review'))).toEqual([['spec/x', '#/o/a/prs/spec%2Fx']]);
    expect(entries(section(view, 'answer'))).toEqual([
      ['Title q', '#/o/a/backlog/q'],
      ['Title h', '#/o/a/backlog/h'],
    ]);
    expect(buttons(section(view, 'answer'))).toEqual([['yes', 'no'], ['Done']]);
    expect(entries(section(view, 'flagged'))).toEqual([['c', '#/o/a/capabilities/c']]);
    expect(entries(section(view, 'session'))).toEqual([['Title r', '#/o/a/backlog/r']]);
    // req:web.views/answer-buttons@c08a8031: no other entry offers a button.
    expect(buttons(section(view, 'session'))).toEqual([[]]);
    expect(view.querySelectorAll('button')).toHaveLength(3);
    expect(section(view, 'session')!.textContent).toContain('/clm:triage');
    expect(view.textContent).not.toContain('Title i');
    expect(homeCount(snap, reviews())).toBe(3);
  });

  it('flagged architectural constraints are listed with the capabilities', () => {
    const arch = architecture({ architecture: [constraint('e', 5, { flagged: true }), constraint('g', 0)] });
    const view = renderHome({ snap: snapshot(), reviews: [], caps: unflagged(), arch, ...links });
    expect(entries(section(view, 'flagged'))).toEqual([['e', '#/o/a/architecture/e']]);
  });

  it('nothing waits: it says so and lists no section', () => {
    const snap = snapshot({ backlog: [item('i', 'idea'), item('b', 'bug')] });
    const view = renderHome({ snap, reviews: [review({ review: false })], caps: unflagged(), arch: noneFlagged(), ...links });
    expect(view.querySelectorAll('.hsec')).toHaveLength(0);
    expect(view.textContent).toContain('Nothing waits on you');
    expect(homeCount(snap, [review({ review: false })])).toBe(0);
  });

  it('counts only the reviews that have arrived, and draws without the reports', () => {
    const view = renderHome({ snap: waiting(), reviews: [], caps: null, arch: null, ...links });
    expect(section(view, 'review')).toBeNull();
    expect(section(view, 'flagged')).toBeNull();
    expect(entries(section(view, 'answer'))).toHaveLength(2);
    expect(view.textContent).not.toContain('Nothing waits on you');
    expect(homeCount(waiting(), [])).toBe(2);
  });
});
