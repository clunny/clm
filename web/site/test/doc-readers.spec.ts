import { test, expect } from '@playwright/test'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { siteDir } from './helpers'

// The marker's text, built so this file isn't itself a reader.
const MARK = 'clm' + ':reads '

/** Globs of the markers in `text`. */
function globs(text: string): string[] {
  return text
    .split(MARK)
    .slice(1)
    .filter((rest) => !/^\s/.test(rest))
    .map((rest) => rest.split(/\s/)[0])
}

/** Whether `glob` names `path`: exactly, as `**`, or as `dir/**` over a path beneath `dir`. */
const covers = (glob: string, path: string) =>
  glob === path || glob === '**' || (glob.endsWith('/**') && path.startsWith(glob.slice(0, -2)))

/** The text of each `build(` call in `text`, up to its closing parenthesis. */
function buildCalls(text: string): string[] {
  const calls: string[] = []
  for (let at = text.indexOf('build('); at >= 0; at = text.indexOf('build(', at + 1)) {
    let depth = 0
    let end = at + 5
    for (; end < text.length; end++) {
      if (text[end] === '(') depth++
      if (text[end] === ')' && --depth === 0) break
    }
    calls.push(text.slice(at, end + 1))
  }
  return calls
}

/** Whether the test reads the repository's guide: it opens the files, serves the build made from
 * them (the `page` fixture or `mainDist`), or builds with no fixture `GUIDE_DIR`. */
const readsGuide = (text: string) =>
  /\brepoDocs\b|\bmainDist\b|\{ page\b/.test(text) ||
  buildCalls(text).some((call) => !call.includes('GUIDE_DIR'))

test('a test that builds the site from the repository guide holds a marker for its pages', () => {
  // req:web.site/doc-readers-marked@730bb707
  const dir = join(siteDir, 'test')
  // This file names what it looks for, and builds and opens nothing.
  const specs = readdirSync(dir).filter((f) => f.endsWith('.spec.ts') && f !== 'doc-readers.spec.ts')
  const readers = specs.filter((f) => readsGuide(readFileSync(join(dir, f), 'utf8')))
  expect(readers).toEqual(expect.arrayContaining(['guide.spec.ts', 'seo.spec.ts']))
  for (const file of readers) {
    const marked = globs(readFileSync(join(dir, file), 'utf8'))
    for (const path of ['docs/guide/quickstart.md', 'docs/guide/README.md', 'docs/spec-format.md']) {
      expect(
        marked.some((g) => covers(g, path)),
        `${file} reads ${path} without a marker for it`,
      ).toBe(true)
    }
  }
})
