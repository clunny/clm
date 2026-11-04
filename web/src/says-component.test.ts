import { readdirSync, readFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { renderImplementation } from './implementation';
import { NOW, implementation, snapshot } from '../test/fixtures';

const SRC = `${dirname(fileURLToPath(import.meta.url))}/`;
const files = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? files(`${dir}${e.name}/`) : [`${dir}${e.name}`]));

/** The old words `component-words` retires, standing alone: no letter, digit or `_` beside them, and not joined by `-`, `/`, `.` or `:` to more of an ID, a path or a key. */
const OLD = /(?<![A-Za-z0-9_\-/])(?<![A-Za-z0-9_][.:])(nodes?|spines?|stubs?)(?![A-Za-z0-9_\-/])(?![.:][A-Za-z0-9_])/gi;

/** Literals that keep an old word, each the exact text and why `component-words` lets it stay. */
const KEEP = new Map<string, string>([
  ['node', 'a CSS class and the blockers\' JSON `kind` value: data and styling hooks, not shown text'],
]);

/** The text of every string literal and template in `src`, as [line, text], skipping comments and `${...}` code. */
function literals(src: string): [number, string][] {
  const out: [number, string][] = [];
  let i = 0;
  let line = 1;
  const skipExpr = () => {
    let depth = 1;
    while (i < src.length && depth > 0) {
      const c = src[i++];
      if (c === '\n') line++;
      else if (c === '{') depth++;
      else if (c === '}') depth--;
      else if (c === "'" || c === '"') str(c, false);
      else if (c === '`') str('`', true);
    }
  };
  const str = (q: string, tpl: boolean) => {
    let buf = '';
    const at = line;
    while (i < src.length && src[i] !== q) {
      const c = src[i++];
      if (c === '\n') line++;
      if (c === '\\') {
        buf += src[i++] ?? '';
      } else if (tpl && c === '$' && src[i] === '{') {
        i++;
        buf += ' ';
        skipExpr();
      } else buf += c;
    }
    i++;
    out.push([at, buf]);
  };
  while (i < src.length) {
    const c = src[i];
    if (c === '/' && src[i + 1] === '/') while (i < src.length && src[i] !== '\n') i++;
    else if (c === '/' && src[i + 1] === '*') {
      i += 2;
      while (i < src.length && !(src[i] === '*' && src[i + 1] === '/')) if (src[i++] === '\n') line++;
      i += 2;
    } else if (c === "'" || c === '"') {
      i++;
      str(c, false);
    } else if (c === '`') {
      i++;
      str('`', true);
    } else {
      if (c === '\n') line++;
      i++;
    }
  }
  return out;
}

// req:web/says-component@88d3bff9
describe('says component', () => {
  it('the Implementation view shows Components and Pending components, and no old word', () => {
    const view = renderImplementation(implementation(), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    const text = [...view.querySelectorAll('*')].map((e) => e.textContent).join(' ');
    expect(text).toContain('Components');
    expect(text).toContain('Pending components');
    expect(text.match(OLD)).toBeNull();
  });

  it('no string literal under web/src holds an old word', () => {
    const bad: string[] = [];
    for (const f of files(SRC).filter((f) => /\.ts$/.test(f) && !/\.test\.ts$/.test(f))) {
      for (const [line, text] of literals(readFileSync(f, 'utf8'))) {
        if (KEEP.has(text)) continue;
        for (const m of text.matchAll(OLD)) bad.push(`${f.slice(SRC.length)}:${line}: ${m[0]}`);
      }
    }
    expect(bad).toEqual([]);
  });
});
