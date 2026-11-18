// @vitest-environment node
// `web/.npmrc` keeps npm from running dependencies' install scripts (specs/web/SPEC.md, Design).
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/** Whether the last line of `npmrc` that sets `ignore-scripts` sets it to `true`. */
const ignoresScripts = (npmrc: string): boolean => {
  const values = npmrc.split('\n').flatMap((line) => {
    const eq = line.indexOf('=');
    if (eq < 0 || line.slice(0, eq).trim() !== 'ignore-scripts') return [];
    return [line.slice(eq + 1).trim()];
  });
  return values.at(-1) === 'true';
};

// req:web/no-install-scripts@d8da6553
describe('web/.npmrc', () => {
  it('ends up with ignore-scripts=true', () => {
    const text = readFileSync(fileURLToPath(new URL('../.npmrc', import.meta.url)), 'utf8');
    expect(ignoresScripts(text)).toBe(true);
  });

  it('a later line can turn scripts back on', () => {
    expect(ignoresScripts('ignore-scripts=true\n')).toBe(true);
    expect(ignoresScripts('')).toBe(false);
    expect(ignoresScripts('ignore-scripts=false\n')).toBe(false);
    expect(ignoresScripts('ignore-scripts=true\nignore-scripts=false\n')).toBe(false);
  });
});
