// Home: what waits on the person (web.views/home-waiting-on-you). Sections in order, each
// left out when empty: Review, Answer, Flagged, Answer in a session.

import { div, el } from './dom';
import type { Architecture, BacklogView, Capabilities, Review, Snapshot } from './types';

export interface HomeInput {
  snap: Snapshot;
  /** The spec branches' reviews that have arrived: only those count. */
  reviews: Review[];
  /** The reports, null until they arrive. */
  caps: Capabilities | null;
  arch: Architecture | null;
  reviewLink: (branch: string) => string;
  itemLink: (slug: string) => string;
  capLink: (slug: string) => string;
  archLink: (slug: string) => string;
}

/** Whether the person can answer the item from the app: a `question` with choices, or a `human-task`. */
const answerable = (b: BacklogView) => b.kind === 'human-task' || (b.kind === 'question' && (b.choices ?? []).length > 0);

/** The reviews that need review and hold no approval. */
const toReview = (reviews: Review[]) => reviews.filter((r) => r.review && r.approvals.length === 0);

/** The number of entries in Review and Answer: what the sidebar and tab bar badge Home with. */
export function homeCount(snap: Snapshot, reviews: Review[]): number {
  return toReview(reviews).length + snap.backlog.filter(answerable).length;
}

function section(name: string, title: string, entries: HTMLElement[], note?: string): HTMLElement | '' {
  if (!entries.length) return '';
  const s = div('hsec card', el('h2', {}, title), note ? div('note', note) : '', div('hents', ...entries));
  s.dataset.section = name;
  return s;
}

const entry = (href: string, text: string) => div('hent-row', el('a', { class: 'hent', href }, text));

export function renderHome(i: HomeInput): HTMLElement {
  const { snap } = i;
  const review = toReview(i.reviews).map((r) => entry(i.reviewLink(r.branch), r.branch));
  const answer = snap.backlog.filter(answerable).map((b) => entry(i.itemLink(b.slug), b.title));
  const flagged = [
    ...(i.caps?.report.capabilities ?? []).filter((c) => c.flagged).map((c) => entry(i.capLink(c.slug), c.slug)),
    ...(i.arch?.architecture ?? []).filter((c) => c.flagged).map((c) => entry(i.archLink(c.slug), c.slug)),
  ];
  const session = snap.backlog.filter((b) => b.kind === 'question' && !answerable(b)).map((b) => entry(i.itemLink(b.slug), b.title));
  const sections = [
    section('review', 'Review', review),
    section('answer', 'Answer', answer),
    section('flagged', 'Flagged', flagged),
    section('session', 'Answer in a session', session, 'These are answered in a /clm:triage session.'),
  ];
  return div(
    'home',
    div('crumb', snap.repo),
    div('head', div('', el('h1', {}, 'Home'), div('meta', 'What waits on you'))),
    sections.every((s) => s === '') ? div('empty', 'Nothing waits on you.') : sections,
  );
}
