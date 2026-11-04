// Spec PRs: the `spec/*` branches, then each one's review: the intent first, beside
// Approve, then per node every changed element with word marks, an impact summary,
// decisions and backlog edits (web/spec-branch-review, web/review-intent-first,
// web/pr-document).
// Open in full leads to web/full-document.

import { wordMarks } from './diff';
import { Child, chip, code, div, el } from './dom';
import { person, textForm } from './forms';
import { md } from './markdown';
import type { Target } from './route';
import { when } from './time';
import type { BranchApproval, Document, Element, NodeChanges, Review, SpecBranch, Texts } from './types';

const STATUS_LABEL: Record<string, string> = { added: 'Added', modified: 'Modified', removed: 'Removed' };

/** The spec branches, each by name with its page's number and title when it has one. */
export function renderPrList(branches: SpecBranch[], link: (t: Target) => string): HTMLElement {
  return div(
    'prs',
    div('crumb', 'Spec PRs'),
    div('head', div('', el('h1', {}, 'Spec PRs'), div('meta', `${branches.length} spec ${branches.length === 1 ? 'branch' : 'branches'}`))),
    branches.length
      ? div(
          'card',
          ...branches.map((b) =>
            el(
              'div',
              { class: 'prl', 'data-branch': b.branch },
              el('a', { class: 'open', href: link({ branch: b.branch }) }, el('b', { style: 'color:var(--text)' }, b.branch)),
              b.page
                ? div('meta', el('a', { class: 'page', href: b.page.url, target: '_blank', rel: 'noopener' }, `#${b.page.number}`), ` ${b.page.title}`)
                : '',
            ),
          ),
        )
      : div('empty', 'No spec branches.'),
  );
}

function status(t: Texts): string {
  return t.before === null ? 'added' : t.after === null ? 'removed' : 'modified';
}

/** Lines of context kept on each side of a changed element. */
const CONTEXT = 2;

/** The non-blank lines just before and just after `t` in the file `doc`, as it reads at the
 * head (or at the base, for a removed element); null where `t` can't be found. */
export function around(doc: Document | undefined, t: Texts): [string | null, string | null] {
  const text = (t.after ?? t.before ?? '').trim();
  const file = t.after !== null ? doc?.after : doc?.before;
  if (!file || !text) return [null, null];
  const at = file.indexOf(text);
  if (at < 0) return [null, null];
  const lines = (s: string) => s.split('\n').filter((l) => l.trim() !== '');
  const before = lines(file.slice(0, at)).slice(-CONTEXT);
  const after = lines(file.slice(at + text.length)).slice(0, CONTEXT);
  return [before.length ? before.join('\n') : null, after.length ? after.join('\n') : null];
}

function card(key: string, title: string, t: Texts, doc?: Document, extra: Child[] = []): HTMLElement {
  const s = status(t);
  const [pre, post] = around(doc, t);
  return el(
    'div',
    { class: 'req', 'data-el': key },
    div('rh', el('b', {}, title), chip(s, STATUS_LABEL[s])),
    div('rb', pre ? div('ctx', pre) : '', div('', ...wordMarks(t.before, t.after)), post ? div('ctx', post) : ''),
    ...extra,
  );
}

function asset(e: Element): HTMLElement {
  const path = e.path ?? '';
  const side = e.after ?? e.before ?? '';
  if (/\.html?$/i.test(path)) {
    // Previewed with no script and no access to the app's origin.
    const frame = el('iframe', { sandbox: '', srcdoc: side, title: path, loading: 'lazy', referrerpolicy: 'no-referrer' });
    const s = status(e);
    return el('div', { class: 'req', 'data-el': path }, div('rh', el('b', {}, `assets/${path}`), chip(s, STATUS_LABEL[s])), div('rb', frame));
  }
  return card(path, `assets/${path}`, e);
}

/** The requirement IDs the review shows a card for. */
function carded(r: Review): Set<string> {
  return new Set(r.nodes.flatMap((n) => [...n.requirements.added, ...n.requirements.modified, ...n.requirements.removed].map((e) => e.id ?? '')));
}

/** A comment or a change request, by the person part of its writer, with its time. */
function feedback(writer: string, at: string, text: string, mark: string | null): HTMLElement {
  return el(
    'div',
    { class: mark ? 'fb dim' : 'fb' },
    div('fbh', el('b', {}, person(writer)), ` · ${when(at)}`, mark ? [' ', chip('mute', mark)] : ''),
    div('fbt', text),
  );
}

function nodeSection(n: NodeChanges, doc: Document | undefined, fullHref: string | null, extra: (id: string) => Child[]): HTMLElement {
  const parts: HTMLElement[] = [];
  for (const k of ['frontmatter', 'contract', 'design'] as const) {
    const t = n[k];
    if (t) parts.push(card(`${n.id}:${k}`, k[0].toUpperCase() + k.slice(1), t, doc));
  }
  for (const k of ['added', 'modified', 'removed'] as const) for (const r of n.requirements[k]) parts.push(card(r.id ?? '', r.id ?? '', r, doc, extra(r.id ?? '')));
  for (const k of ['added', 'modified', 'removed'] as const) for (const a of n.assets[k]) parts.push(asset(a));
  return div(
    'card sec node-changes',
    div(
      'head',
      el('h3', { class: 'h' }, `${n.id} · ${n.status}`),
      fullHref ? el('a', { class: 'btn open-full', href: fullHref }, 'Open in full') : '',
    ),
    ...parts,
  );
}

/** A node's SPEC.md, as `clm_model` names nodes: the root `clm` is `specs/SPEC.md`, and
 * `a.b` is `specs/a/b/SPEC.md`. */
export function specPath(id: string): string {
  return id === 'clm' ? 'specs/SPEC.md' : `specs/${id.split('.').join('/')}/SPEC.md`;
}

/** A spec branch's review in a repository that counts only signed approvals, which the
 * Worker can't write (web/signed-approvals). A review without `signed` is unsigned. */
function signedRepo(r: Review): boolean {
  return r.signed === true;
}

/** Why Approve can't be pressed, or null when it can. `accepted`: the
 * Worker accepted this page's request. */
export function cannotAct(r: Review, offline: boolean, accepted: boolean): string | null {
  if (signedRepo(r)) return 'This repository counts only signed approvals: approve with clm approve';
  if (accepted || r.approvals.length) return 'Approved';
  if (offline) return 'Offline';
  if (!r.fingerprint) return 'No fingerprint: the branch is too large to pin';
  return null;
}

/** Why a comment (or a request for changes) can't be sent, or null when it can. */
export function cannotWrite(r: Review, offline: boolean): string | null {
  if (offline) return 'Offline';
  if (!r.fingerprint) return 'No fingerprint: the branch is too large to pin';
  return null;
}

export interface ReviewOptions {
  offline: boolean;
  /** The Worker accepted the approval sent from this page. */
  accepted: boolean;
  onAct: () => void;
  /** The in-app link that opens a changed file in full. */
  fullHref: (path: string) => string;
  /** Sends a comment on the intent (`null`) or a requirement: null once it went through,
   * else why not (web/comment-on-review). */
  onComment?: (target: string | null, text: string) => Promise<string | null>;
  /** What the review's form drafts are kept under, such as its Worker path, so the same
   * branch name in another repository has drafts of its own. */
  scope?: string;
  /** Sends a request for changes, answering as `onComment` does (web/request-changes). */
  onRequest?: (text: string) => Promise<string | null>;
}

export function renderReview(r: Review, o: ReviewOptions): HTMLElement {
  const c = r.change;
  const why = cannotAct(r, o.offline, o.accepted);
  const finished = why === 'Approved';
  const act = el(
    'button',
    { type: 'button', class: 'btn pri approve', disabled: why !== null, title: why ?? 'Approve this intent' },
    finished ? why : 'Approve intent',
  );
  act.addEventListener('click', () => {
    if (act.disabled) return;
    // One request per tap; the view is drawn again with its result.
    act.disabled = true;
    o.onAct();
  });

  const comments = r.comments ?? [];
  const requests = r.change_requests ?? [];
  const cards = carded(r);
  const writeWhy = cannotWrite(r, o.offline);
  const commentOn = (target: string | null): Child =>
    textForm({
      key: `${o.scope ?? r.branch}|comment|${target ?? ''}`,
      label: 'Comment',
      cls: 'comment',
      disabled: writeWhy,
      submit: (text) => o.onComment?.(target, text) ?? Promise.resolve('Not sent.'),
    });
  const said = (target: string | null) =>
    comments
      .filter((m) => (target === null ? m.target === null || !cards.has(m.target) : m.target === target))
      .map((m) => {
        // A comment on a requirement this review shows no card for names it.
        const on = target === null && m.target !== null ? `on ${m.target}` : null;
        const marks = [on, m.current ? null : 'on an earlier version'].filter((x) => x !== null).join(', ');
        return feedback(m.writer, m.at, m.text, marks || null);
      });
  // A request that no longer holds the branch reads as a comment would.
  const settled = requests.filter((q) => !q.held).map((q) => feedback(q.writer, q.at, q.text, q.current ? 'since approved' : 'on an earlier version'));

  const intent = div(
    'card sec intent',
    el('h3', { class: 'h' }, 'Intent'),
    c ? md(c.intent_section || c.intent) : div('note', 'This change has no change.md.'),
    ...said(null),
    ...settled,
    commentOn(null),
  );

  const nodes = r.nodes.map((n) => {
    const d = r.documents.find((doc) => doc.path === specPath(n.id));
    return nodeSection(n, d, d ? o.fullHref(d.path) : null, (id) => [...said(id), commentOn(id)]);
  });

  const count = (k: 'added' | 'modified' | 'removed') => r.nodes.reduce((s, n) => s + n.requirements[k].length, 0);
  const reqSummary = (['added', 'modified', 'removed'] as const)
    .map((k) => [count(k), k] as const)
    .filter(([n]) => n)
    .map(([n, k]) => `${n} ${k}`)
    .join(', ');
  const records = (title: string, list: Review['decisions'], key: 'name' | 'slug') =>
    list.length
      ? div(
          'sec',
          el('h3', { class: 'h' }, title),
          ...list.map((d) =>
            el(
              'div',
              { class: 'dec', [`data-${key}`]: d[key] ?? '' },
              code(d[key] ?? ''),
              ' ',
              chip(d.status, d.status),
              el('details', {}, el('summary', {}, d.status === 'removed' ? 'What it said' : 'What it says'), md(d.after ?? d.before ?? '')),
            ),
          ),
        )
      : '';
  const signed = signedRepo(r);
  const listed: BranchApproval[] = r.approvals;
  // In a signed repository an approval is drawn neutral, marked by whether `clm land` will
  // check it: the app never calls one counted.
  const by = !listed.length
    ? [chip('mute', 'not approved')]
    : !signed
      ? listed.map((a) => chip('ok', `Approved by ${a.writer} · ${when(a.at)}`))
      : [
          ...listed.map((a) => [
            chip('mute', `Approved by ${a.writer} · ${when(a.at)}`),
            a.signed ? chip('sky sig', 'signed') : chip('hard sig', "unsigned, doesn't count"),
          ]),
          listed.some((a) => a.signed) ? '' : chip('hard', 'no signed approval'),
        ];
  // A request that holds the branch: a land waits on the author's push or the requester's
  // approval, as `clm land` holds it.
  const holding = requests
    .filter((q) => q.held)
    .map((q) =>
      div(
        'changes-held',
        div('fbh', el('b', {}, `Changes requested by ${person(q.writer)}`), ` · ${when(q.at)}`),
        div('fbt', q.text),
        div('note', `The branch won't land until the author pushes a change or ${person(q.writer)} approves it.`),
      ),
    );
  // Beside Approve, and usable where Approve isn't: a request withholds a land.
  const request: Child = textForm({
    key: `${o.scope ?? r.branch}|request`,
    label: 'Request changes',
    cls: 'request',
    disabled: writeWhy,
    submit: (text) => o.onRequest?.(text) ?? Promise.resolve('Not sent.'),
  });
  const command = `clm approve ${r.branch}`;
  const copy = el('button', { type: 'button', class: 'btn copy', title: 'Copy the command' }, 'Copy');
  copy.addEventListener('click', () => void navigator.clipboard?.writeText(command).catch(() => undefined));
  const signedNote = signed
    ? div('note signed-note', "This repository counts only approvals signed with the reviewer's SSH key. Approve from your clone with ", code(command), ' ', copy)
    : '';

  return div(
    'review',
    div('crumb', 'Spec PRs / ', r.branch),
    div(
      'head',
      div(
        '',
        el('h1', {}, r.branch),
        div(
          'meta',
          r.review ? chip('hard', 'needs review') : '',
          ...by,
          chip(r.checked ? 'ok' : 'mute', r.checked ? 'checked' : 'not checked'),
          r.page ? el('a', { class: 'page', href: r.page.url, target: '_blank', rel: 'noopener' }, `#${r.page.number}`) : '',
          ...r.nodes.map((n) => chip('node', n.id)),
          ...(c?.resolves.length ? ['resolves ', ...c.resolves.map(code)] : []),
          code(`${r.branch} → ${r.base}`),
        ),
        ...holding,
      ),
    ),
    div('prhead', div('', intent), div('approve-bar', act, request, signedNote)),
    div(
      'doc',
      div('', ...(nodes.length ? nodes : [div('card sec', div('note', 'No component changes.'))])),
      div(
        'aside card',
        div(
          'sec',
          el('h3', { class: 'h' }, 'Impact'),
          div('kv', 'Components touched', el('b', {}, String(r.nodes.length))),
          div('kv', 'Requirements', el('b', {}, reqSummary || 'none')),
          div('kv', 'Contracts', el('b', {}, String(r.nodes.filter((n) => n.contract).length))),
          div('kv', 'Designs', el('b', {}, String(r.nodes.filter((n) => n.design).length))),
        ),
        records('Decisions', r.decisions, 'name'),
        records('Backlog', r.backlog, 'slug'),
        r.documents.length
          ? div(
              'sec',
              el('h3', { class: 'h' }, 'Files'),
              ...r.documents.map((d) => div('kv', el('a', { href: o.fullHref(d.path), class: 'mono' }, d.path))),
            )
          : '',
      ),
    ),
  );
}
