// The forms that write to the person's web ledger: a comment, a request for changes
// (web/comment-on-review, web/request-changes) and a proposal (web/propose-from-backlog).
// Each sends once per tap and nothing while a send is in flight, and keeps what was typed
// across the redraws a refresh makes, until it is sent.

import { div, el } from './dom';

/** What was typed into an open form, not yet sent. */
export interface Draft {
  text: string;
  title?: string;
}

/** The open forms' drafts, by form key: a form is open while it has one. */
export const drafts = new Map<string, Draft>();

/** The person part of a record's writer, `<person>/<clone>`. */
export function person(writer: string): string {
  return writer.split('/')[0];
}

/** Wires `send` to run `go` once per tap and none while it runs. `go` answers null when it
 * went through, which closes the form, or the reason it didn't, shown in `err` with the form
 * kept open. */
export function sendOnce(send: HTMLButtonElement, err: HTMLElement, go: () => Promise<string | null>, closed: () => void) {
  let busy = false;
  send.addEventListener('click', async () => {
    if (busy || send.disabled) return;
    busy = true;
    send.disabled = true;
    err.textContent = '';
    const why = await go().catch(() => 'Not sent: offline.');
    busy = false;
    send.disabled = false;
    if (why === null) closed();
    else err.textContent = why;
  });
}

/** Sends the draft under `key` with `send`, or says what is missing when `send` is null.
 * The draft is dropped while it is sent, so a view drawn again on success shows the form
 * closed, and is put back when the send didn't go through. */
export async function submitDraft(key: string, draft: Draft, send: (() => Promise<string | null>) | null): Promise<string | null> {
  if (!send) return 'Write something first.';
  drafts.delete(key);
  const why = await send().catch(() => 'Not sent: offline.');
  if (why !== null) drafts.set(key, draft);
  return why;
}

export interface TextForm {
  /** The form's key in `drafts`. */
  key: string;
  /** The toggle's label, such as Comment. */
  label: string;
  cls: string;
  /** Why the form can't be used, or null when it can. */
  disabled: string | null;
  /** Sends the text, answering as `sendOnce`'s `go` does. */
  submit: (text: string) => Promise<string | null>;
}

/** A toggle that opens a one-field form, open at once when `drafts` holds its key. */
export function textForm(o: TextForm): HTMLElement {
  const wrap = div(`fbw ${o.cls}`);
  const toggle = el('button', { type: 'button', class: `btn ${o.cls}`, disabled: o.disabled !== null, title: o.disabled ?? o.label }, o.label);
  const draw = () => {
    const draft = drafts.get(o.key);
    if (!draft) return wrap.replaceChildren(toggle);
    const text = el('textarea', { rows: 3, 'aria-label': o.label, 'data-draft': o.key });
    text.value = draft.text;
    text.addEventListener('input', () => (draft.text = text.value));
    const send = el('button', { type: 'button', class: 'btn pri send', disabled: o.disabled !== null, title: o.disabled ?? 'Send' }, 'Send');
    const cancel = el('button', { type: 'button', class: 'btn cancel' }, 'Cancel');
    cancel.addEventListener('click', () => {
      drafts.delete(o.key);
      draw();
    });
    const err = div('err');
    sendOnce(send, err, () => submitDraft(o.key, draft, text.value.trim() ? () => o.submit(text.value.trim()) : null), draw);
    wrap.replaceChildren(div('fbform', text, div('fbact', send, cancel), err));
    text.focus();
  };
  toggle.addEventListener('click', () => {
    if (toggle.disabled) return;
    drafts.set(o.key, { text: '' });
    draw();
  });
  draw();
  return wrap;
}
