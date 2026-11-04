// req:plugin.panel/capabilities-tab@fd439d25
import { describe, expect, test } from 'vitest'

import { capabilitiesPane, capabilityRows, capabilityView } from '../src/capabilities.ts'
import { type PaneLine, keyTarget } from '../src/inspector.ts'
import { type Run, type Span, TABS, ansiLines, isViewTab } from '../src/panel.ts'

const ok = (stdout: string): Run => ({ exitCode: 0, stdout, stderr: '' })
const failed = (stderr: string): Run => ({ exitCode: 1, stdout: '', stderr })
const text = (spans: readonly Span[]) => spans.map(s => s.text).join('')
const texts = (lines: readonly PaneLine[]) => lines.map(l => text(l.spans))
const keys = (lines: readonly PaneLine[]) => lines.flatMap(l => (l.key ? [l.key] : []))

const LIST =
  'capabilities 2 · 1 ⚑ · costs are estimates: a node\'s cost is split by its rules\' share\n' +
  '  adopt-clm  ≈$1.20  85k tok  9 commits  2 reviews  0 workarounds  never decided\n' +
  '⚑ git-host   ≈$0.40  12k tok  3 commits  1 reviews  3 workarounds  never decided\n' +
  'unattributed       ≈$0.10  2k tok\n' +
  'architecture only  ≈$0.00  0 tok\n'
const report = (...slugs: string[]) =>
  ok(JSON.stringify({ capabilities: slugs.map(slug => ({ id: `capability/${slug}`, slug })), unattributed: {} }))
const REPORT = report('adopt-clm', 'git-host')
const PAGE =
  'git-host\n' +
  'A person can land through GitHub, a bare server or no host.\n' +
  'rules\n' +
  '  host/github\n' +
  'decisions\n' +
  '  never decided\n' +
  'decide: clm capabilities decide git-host --value high|medium|low --decision keep|cut|redesign|accept\n'

describe('capabilities tab', () => {
  test('capabilities tab runs clm capabilities', () => {
    expect(TABS).toEqual(['spec', 'flight', 'blockers', 'capabilities'])
    // No view: no width, no SVG, whatever the surface.
    expect(isViewTab('capabilities')).toBe(false)
    // The list and its report, started together.
    expect(capabilitiesPane(null)).toEqual({ args: ['capabilities'], report: ['capabilities', '--format', 'json'] })
    const drawn = capabilityView({ slug: null, run: ok(LIST), report: REPORT })
    expect(texts(drawn.lines)).toEqual(LIST.trimEnd().split('\n'))
    expect(drawn.back).toBe(false)
  })

  test('rows map to slugs by position', () => {
    const rows = capabilityRows(ansiLines(LIST), REPORT)
    expect(rows).toEqual(
      new Map([
        [1, 'adopt-clm'],
        [2, 'git-host'],
      ]),
    )
    const drawn = capabilityView({ slug: null, run: ok(LIST), report: REPORT })
    expect(drawn.lines.map(l => l.key)).toEqual([
      undefined,
      'capability:adopt-clm',
      'capability:git-host',
      undefined,
      undefined,
    ])
    expect(keyTarget('capability:git-host')).toEqual({ capability: 'git-host' })
  })

  test('count mismatch draws nothing pressable', () => {
    for (const json of [
      report('adopt-clm', 'git-host', 'keep-a-backlog'),
      // The same count, but a capability renamed between the two runs.
      report('adopt-clm', 'keep-a-backlog'),
      failed('boom'),
      ok('not json'),
      ok('{"capabilities": 3}'),
      undefined,
    ]) {
      expect(capabilityRows(ansiLines(LIST), json)).toBeNull()
      const drawn = capabilityView({ slug: null, run: ok(LIST), report: json })
      expect(keys(drawn.lines)).toEqual([])
      expect(texts(drawn.lines)).toEqual(LIST.trimEnd().split('\n'))
    }
  })

  test('pressing a row shows its page with Back', () => {
    // A press sets the `capability` atom to the row's slug; the tab then runs its page alone.
    expect(capabilitiesPane('git-host')).toEqual({ args: ['capabilities', 'git-host'] })
    const page = capabilityView({ slug: 'git-host', run: ok(PAGE) })
    expect(page.back).toBe(true)
    expect(texts(page.lines)).toEqual(PAGE.trimEnd().split('\n'))
    expect(keys(page.lines)).toEqual([])
  })

  test('no control runs decide', () => {
    const page = capabilityView({ slug: 'git-host', run: ok(PAGE) })
    expect(texts(page.lines).at(-1)).toMatch(/^decide: clm capabilities decide git-host/)
    const list = capabilityView({ slug: null, run: ok(LIST), report: REPORT })
    // The only presses are the rows (a slug) and Back: none names decide.
    for (const drawn of [page, list]) {
      for (const key of keys(drawn.lines)) {
        expect(key).toMatch(/^capability:[a-z0-9-]+$/)
        expect(key).not.toContain('decide')
      }
    }
    // `clm capabilities` reads `decide` and `help` as subcommands: rows so slugged aren't
    // pressable, and the rest are.
    const odd = ansiLines('h\n  decide    ≈$0.00\n  help      ≈$0.00\n  git-host  ≈$0.00\nunattributed  ≈$0.00\n')
    expect(capabilityRows(odd, report('decide', 'help', 'git-host'))).toEqual(new Map([[3, 'git-host']]))
  })

  test('stderr on failure', () => {
    expect(capabilityView({ slug: null, run: failed('boom'), report: REPORT })).toEqual({
      back: false,
      error: 'boom',
      lines: [],
    })
    // A page asked for keeps Back, so a slug kept across a reload that no longer exists can be left.
    expect(capabilityView({ slug: 'gone', run: failed('no capability gone') })).toEqual({
      back: true,
      error: 'no capability gone',
      lines: [],
    })
  })
})
