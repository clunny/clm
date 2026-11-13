// Markdown from the repo, rendered and then sanitised: anyone who can open a PR writes it,
// and it is shown where the session cookie can approve.

import DOMPurify from 'dompurify';
import { marked } from 'marked';

export function md(text: string): HTMLElement {
  const html = marked.parse(text, { async: false, gfm: true });
  const e = document.createElement('div');
  e.className = 'md';
  e.innerHTML = DOMPurify.sanitize(html, { USE_PROFILES: { html: true } });
  return e;
}
