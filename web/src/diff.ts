// Word-level marks between an element's text before and after, computed in the browser.

import { diffWords } from 'diff';
import { el } from './dom';

export function wordMarks(before: string | null, after: string | null): Node[] {
  return diffWords(before ?? '', after ?? '').map((part) => {
    if (part.added) return el('ins', {}, part.value);
    if (part.removed) return el('del', {}, part.value);
    return document.createTextNode(part.value);
  });
}
