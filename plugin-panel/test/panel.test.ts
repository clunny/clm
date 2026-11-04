import ts from 'typescript'
import { describe, expect, test } from 'vitest'

import {
  FETCH_ARGV,
  FIRST_TAB,
  ORIGIN_MAIN_ARGV,
  NOT_A_REPO,
  NOT_INSTALLED,
  PANE_TITLE,
  REDRAW_MS,
  TABS,
  type Host,
  type Run,
  ansiLines,
  clmCommand,
  clmRoot,
  command,
  fetchAge,
  fetchIfDue,
  fetchWarning,
  hasOriginMain,
  isViewTab,
  paneBody,
  runClm,
  sessionDirGone,
  unfenced,
  viewArgs,
} from '../src/panel.ts'
import { chartLines, modelRuns } from '../src/inspector.ts'

const PANEL = '/plugins/clm/plugin-panel'
const SHIM = '/plugins/clm/plugin/bin/clm'

type Fake = Host & { calls: { argv: readonly string[]; cwd?: string }[] }

/** A `$` over a fake file system (`files` and `dirs` by absolute path) whose processes answer `answer`. */
function fake(opts: {
  files?: string[]
  dirs?: string[]
  root?: string
  answer?: (argv: readonly string[]) => Run
  /** What starting a process throws when there is no `answer`. */
  startError?: string
}): Fake {
  const files = new Set(opts.files ?? [])
  const dirs = new Set(opts.dirs ?? [])
  const calls: Fake['calls'] = []
  return {
    calls,
    plugin: { root: opts.root ?? PANEL },
    process: {
      async run(argv, init) {
        calls.push({ argv, cwd: init?.cwd })
        if (!opts.answer) throw new Error(opts.startError ?? `spawn ${argv[0]} ENOENT`)
        return opts.answer(argv)
      },
    },
    fs: {
      async stat(path) {
        if (files.has(path)) return { kind: 'file' as const }
        if (dirs.has(path)) return { kind: 'dir' as const }
        throw new Error(`ENOENT ${path}`)
      },
      async list(path) {
        const prefix = `${path}/`
        const names = [...dirs]
          .filter(d => d.startsWith(prefix) && !d.slice(prefix.length).includes('/'))
          .map(d => ({ name: d.slice(prefix.length), kind: 'dir' }))
        if (names.length === 0 && !dirs.has(path ?? '')) throw new Error(`ENOENT ${path}`)
        return names
      },
    },
  }
}

const REPO = { files: ['/work/repo/specs/SPEC.md', SHIM], dirs: ['/work/repo', '/work/repo/.git'] }
const ok = (stdout: string): Run => ({ exitCode: 0, stdout, stderr: '' })

describe('view-command-in-chat', () => {
  // req:plugin.panel/view-command-in-chat@35f95b15
  test('a view in the chat is clm view <name> --at origin/main at the cwd, in a code block', async () => {
    const $ = fake({ ...REPO, answer: () => ok('flight  2 nodes pending\n') })
    const answer = await command($, 'flight', '/work/repo/crates')
    expect(answer).toEqual({ text: '\n```\nflight  2 nodes pending\n```' })
    expect($.calls).toEqual([
      { argv: ORIGIN_MAIN_ARGV, cwd: '/work/repo/crates' },
      { argv: [SHIM, 'view', 'flight', '--at', 'origin/main'], cwd: '/work/repo/crates' },
    ])
    expect('text' in answer && unfenced(`clm-panel: ${answer.text}`)).toBe('flight  2 nodes pending')
    expect(unfenced('error: bad spec')).toBeUndefined()
  })

  test('status is clm status; every view name is accepted', async () => {
    const $ = fake({ ...REPO, answer: argv => ok(argv.slice(1).join(' ')) })
    for (const name of ['status', 'spec', 'flight', 'blockers']) {
      const answer = await command($, ` ${name} `, '/work/repo')
      expect(answer).toEqual({ text: `\n\`\`\`\n${name === 'status' ? 'status' : `view ${name} --at origin/main`}\n\`\`\`` })
    }
  })

  test('with no origin/main, or no git, the view is of the checkout, with no --at', async () => {
    for (const rev of [{ exitCode: 1, stdout: '', stderr: '' }, undefined]) {
      const $ = fake({
        ...REPO,
        answer: argv => {
          if (argv[0] !== 'git') return ok('chart\n')
          if (rev === undefined) throw new Error('spawn git ENOENT')
          return rev
        },
      })
      expect(await command($, 'flight', '/work/repo')).toEqual({ text: '\n```\nchart\n```' })
      expect($.calls.at(-1)?.argv).toEqual([SHIM, 'view', 'flight'])
    }
  })

  test('a failing clm answers with its stderr', async () => {
    const $ = fake({ ...REPO, answer: () => ({ exitCode: 1, stdout: 'x', stderr: 'error: bad spec' }) })
    expect(await command($, 'spec', '/work/repo')).toEqual({ text: 'error: bad spec' })
  })

  test('an unknown name lists the names and runs nothing', async () => {
    const $ = fake({ ...REPO, answer: () => ok('') })
    const answer = await command($, 'everything', '/work/repo')
    expect('text' in answer && answer.text).toContain('status, spec, flight, blockers')
    expect($.calls).toEqual([])
  })
})

describe('views-panel', () => {
  // req:plugin.panel/views-panel@99ca88c7
  test('the pane is CLM with Spec, Flight, Blockers and Capabilities, Flight first', () => {
    expect(PANE_TITLE).toBe('CLM')
    expect(TABS).toEqual(['spec', 'flight', 'blockers', 'capabilities'])
    expect(FIRST_TAB).toBe('flight')
  })

  test('the terminal pane asks for ANSI at its width; others for SVG', () => {
    expect(viewArgs('flight', 'terminal', 80)).toEqual(['view', 'flight', '--format', 'ansi', '--width', '80'])
    for (const surface of ['desktop', 'vscode', 'mobile'] as const) {
      expect(viewArgs('spec', surface, 80)).toEqual(['view', 'spec', '--format', 'svg'])
    }
  })

  test('every clm view the pane runs passes --at origin/main while the ref exists, and none otherwise', async () => {
    expect(viewArgs('flight', 'terminal', 80, true)).toEqual([
      'view', 'flight', '--at', 'origin/main', '--format', 'ansi', '--width', '80',
    ])
    expect(viewArgs('spec', 'desktop', 80, true)).toEqual(['view', 'spec', '--at', 'origin/main', '--format', 'svg'])
    expect(modelRuns('spec', 'terminal', 'a', true)).toEqual({
      own: ['view', 'spec', '--at', 'origin/main', '--format', 'json'],
      other: ['view', 'flight', '--at', 'origin/main', '--format', 'json'],
    })
    // No origin/main: the checkout's view, with no `--at`.
    expect(viewArgs('flight', 'terminal', 80, false)).toEqual(['view', 'flight', '--format', 'ansi', '--width', '80'])
    expect(modelRuns('flight', 'terminal', null, false).own).toEqual(['view', 'flight', '--format', 'json'])
    // Whether the ref exists is `git rev-parse --verify -q` at the cwd; git failing to start reads as no.
    const has = fake({ ...REPO, answer: () => ok('abc\n') })
    expect(await hasOriginMain(has, '/work/repo')).toBe(true)
    expect(has.calls).toEqual([{ argv: ['git', 'rev-parse', '--verify', '-q', 'refs/remotes/origin/main'], cwd: '/work/repo' }])
    expect(await hasOriginMain(fake({ ...REPO, answer: () => ({ exitCode: 1, stdout: '', stderr: '' }) }), '/work/repo')).toBe(false)
    expect(await hasOriginMain(fake(REPO), '/work/repo')).toBe(false)
    await expect(hasOriginMain(fake({ ...REPO, startError: 'The operation was aborted' }), '/work/repo')).rejects.toThrow('aborted')
  })

  test("a chart at origin/main begins with the line naming its commit, which no node line is", () => {
    const spec = ansiLines('origin/main abc1234 · fetched 3m ago · checkout impl/a\nr  active\n')
    const at = ok(JSON.stringify({ at: { rev: 'origin/main', commit: 'abc1234' }, nodes: [{ node: 'r' }] }))
    expect(chartLines({ tab: 'spec', chart: spec, own: at, selected: null, unfolded: [] }).lines.map(l => l.key)).toEqual([
      undefined,
      'node:r',
    ])
  })

  test("ANSI escapes become coloured runs", () => {
    const lines = ansiLines('\x1b[34mflight\x1b[0m  2 nodes\n  ci  \x1b[36mready\x1b[0m\n\n\x1b[1;31mx\x1b[22my\x1b[39mz\x1b[K\n')
    expect(lines).toEqual([
      [{ text: 'flight', color: 'blue' }, { text: '  2 nodes' }],
      [{ text: '  ci  ' }, { text: 'ready', color: 'cyan' }],
      [],
      [{ text: 'x', color: 'red', bold: true }, { text: 'y', color: 'red' }, { text: 'z' }],
    ])
  })

  test('it fetches at most once every 2 minutes, however often it redraws', async () => {
    const $ = fake({ ...REPO, answer: () => ok('') })
    let last: number | undefined
    // A turn ends at 0s, the timer fires at 30s, three turns end within the next minute.
    for (const now of [0, REDRAW_MS, 40_000, 55_000, 70_000, 90_000, 119_999]) {
      last = fetchIfDue($, '/work/repo', now, last).lastFetch
    }
    expect($.calls).toEqual([{ argv: FETCH_ARGV, cwd: '/work/repo' }])
    fetchIfDue($, '/work/repo', 120_000, last)
    expect($.calls).toHaveLength(2)
  })

  test("a failed fetch still waits 2 minutes", async () => {
    const $ = fake(REPO)
    const first = fetchIfDue($, '/work/repo', 0, undefined)
    await first.fetching
    expect(fetchIfDue($, '/work/repo', 60_000, first.lastFetch)).toEqual({ lastFetch: 0 })
    expect($.calls).toHaveLength(1)
  })

  test('a failed fetch draws an amber line above the chart until a fetch succeeds', async () => {
    const failing = fake({
      ...REPO,
      answer: () => ({ exitCode: 128, stdout: '', stderr: "fatal: unable to access 'https://x/': Could not resolve host\nmore\n" }),
    })
    const first = await fetchIfDue(failing, '/work/repo', 60_000, undefined).fetching
    expect(first).toEqual({ startedAt: 60_000, exitCode: 128, stderr: expect.stringContaining('fatal: unable to access') })
    expect(fetchWarning(first, 60_000 + 3 * 60_000 + 59_000)).toEqual([
      { text: "⚠ fetch failed 3m ago: fatal: unable to access 'https://x/': Could not resolve host", color: 'yellow' },
    ])
    const later = await fetchIfDue(fake({ ...REPO, answer: () => ok('') }), '/work/repo', 200_000, first?.startedAt).fetching
    expect(later).toMatchObject({ exitCode: 0 })
    expect(fetchWarning(later, 210_000)).toBeUndefined()
    expect(fetchWarning(undefined, 0)).toBeUndefined()
    // A git that can't start failed too, saying why.
    const none = await fetchIfDue(fake(REPO), '/work/repo', 0, undefined).fetching
    expect(fetchWarning(none, 0)?.[0]?.text).toBe('⚠ fetch failed 0m ago: spawn git ENOENT')
    // One that failed saying nothing still says how; one the engine cut short didn't fail.
    expect(fetchWarning({ startedAt: 0, exitCode: 1, stderr: '' }, 0)?.[0]?.text).toBe('⚠ fetch failed 0m ago: git exited 1')
    expect(await fetchIfDue(fake({ ...REPO, startError: 'The operation was aborted' }), '/work/repo', 0, undefined).fetching).toBeUndefined()
  })

  test('the age is whole minutes under an hour, whole hours from then on', () => {
    expect([0, 59_999, 60_000, 59 * 60_000 + 59_999, 3_600_000, 7_199_999, 7_200_000].map(ms => fetchAge(ms))).toEqual([
      '0m',
      '0m',
      '1m',
      '59m',
      '1h',
      '1h',
      '2h',
    ])
  })

  test('two overlapping redraws start one fetch', async () => {
    const $ = fake({ ...REPO, answer: () => ok('') })
    const a = fetchIfDue($, '/work/repo', 0, undefined)
    fetchIfDue($, '/work/repo', 1, a.lastFetch)
    expect($.calls).toHaveLength(1)
  })

  test('the pane body: stderr on failure, else the ANSI lines on the terminal or the SVG', () => {
    expect(paneBody('terminal', { exitCode: 1, stdout: 'x', stderr: 'violations: 2' })).toEqual({ error: 'violations: 2' })
    expect(paneBody('terminal', ok('\x1b[34mx\x1b[0m'))).toEqual({ ansi: [[{ text: 'x', color: 'blue' }]] })
    expect(paneBody('desktop', ok('<svg/>'))).toEqual({ svg: '<svg/>' })
  })

  test('no pictures: no Picture button, no Image, and no png on any tab or surface', () => {
    for (const surface of ['terminal', 'desktop', 'vscode', 'mobile'] as const) {
      for (const tab of ['spec', 'flight', 'blockers'] as const) {
        for (const at of [true, false]) expect(viewArgs(tab, surface, 80, at)).not.toContain('png')
      }
    }
    const panel = `${ts.sys.getCurrentDirectory()}/`
    for (const file of ['hooks/register.tsx', 'src/panel.ts', 'src/inspector.ts', 'src/agents.ts', 'src/capabilities.ts']) {
      expect(ts.sys.readFile(panel + file), file).not.toMatch(/Picture|<Image|\bpng\b/)
    }
  })

  test("the terminal chart's node lines are pressable and the Blockers tab's chains folded", () => {
    expect(modelRuns('blockers', 'terminal', null).own).toEqual(['view', 'blockers', '--format', 'json'])
    const chart = ansiLines('blockers\ngate  holds up 1\n└─ c\n')
    const roots = ok(JSON.stringify({ roots: [{ root: 'gate' }] }))
    const { lines } = chartLines({ tab: 'blockers', chart, own: roots, selected: null, unfolded: [] })
    expect(lines.map(l => l.key)).toEqual([undefined, 'root:gate'])
    const spec = ansiLines('r  active\n')
    const nodes = ok(JSON.stringify({ nodes: [{ node: 'r' }] }))
    expect(chartLines({ tab: 'spec', chart: spec, own: nodes, selected: null, unfolded: [] }).lines[0]!.key).toBe('node:r')
  })

  test('the width is a whole number clm takes', () => {
    expect(viewArgs('flight', 'terminal', 79.5).at(-1)).toBe('79')
    expect(viewArgs('flight', 'terminal', Number.NaN).at(-1)).toBe('100')
    expect(viewArgs('flight', 'terminal', 1e6).at(-1)).toBe('65535')
  })

  test('extended colours and hyperlinks keep their text', () => {
    expect(ansiLines('\x1b[38;2;255;0;34ma\x1b[0m \x1b]8;;https://x\x1b\\link\x1b]8;;\x1b\\')).toEqual([
      [{ text: 'a' }, { text: ' ' }, { text: 'link' }],
    ])
  })

  test('a failure with nothing on stderr still says something', async () => {
    const $ = fake({ ...REPO, answer: () => ({ exitCode: 2, stdout: '', stderr: '' }) })
    expect(await command($, 'flight', '/work/repo')).toEqual({ text: 'clm exited 2' })
  })

  test("a failing clm view shows its stderr", async () => {
    const $ = fake({ ...REPO, answer: () => ({ exitCode: 1, stdout: '', stderr: 'violations: 2' }) })
    const run = await runClm($, viewArgs('flight', 'terminal', 80), '/work/repo')
    expect(run).toEqual({ exitCode: 1, stdout: '', stderr: 'violations: 2' })
  })

  test("outside a CLM repository it says so and opens nothing", async () => {
    const $ = fake({ files: [SHIM, '/work/specs/SPEC.md'], dirs: ['/work/repo', '/work/repo/.git'], answer: () => ok('') })
    expect(await command($, '', '/work/repo/src')).toEqual({ text: NOT_A_REPO })
    expect(await command($, 'flight', '/work/repo')).toEqual({ text: NOT_A_REPO })
    expect($.calls).toEqual([])
  })

  test('inside one, no argument opens the pane', async () => {
    const $ = fake({ ...REPO, answer: () => ok('') })
    expect(await command($, '', '/work/repo/src/deep')).toEqual({ open: true })
  })
})

describe('panel-package', () => {
  // req:plugin.panel/panel-package@e80d5d13
  test("with no clm, it says clm isn't installed, naming the clm plugin", async () => {
    const $ = fake({ files: ['/work/repo/specs/SPEC.md'], dirs: ['/work/repo', '/work/repo/.git'] })
    expect(await clmCommand($)).toBe('clm')
    expect(await command($, 'flight', '/work/repo')).toEqual({ text: NOT_INSTALLED })
    expect(NOT_INSTALLED).toContain('the `clm` plugin')
    // No git either, so no `origin/main`: the checkout's view.
    expect($.calls.at(-1)).toEqual({ argv: ['clm', 'view', 'flight'], cwd: '/work/repo' })
  })

  test("from the plugin cache it runs the clm plugin's shim of its own version", async () => {
    const cache = '/home/u/.claude/plugins/cache/clm'
    const shim = (v: string) => `${cache}/clm/${v}/bin/clm`
    const $ = fake({
      root: `${cache}/clm-panel/0.6.0`,
      files: [shim('0.10.0'), shim('0.6.0'), shim('0.9.0')],
      dirs: [`${cache}/clm/0.10.0`, `${cache}/clm/0.6.0`, `${cache}/clm/0.9.0`],
    })
    expect(await clmCommand($)).toBe(shim('0.6.0'))
    const other = fake({ ...$, root: `${cache}/clm-panel/0.7.0`, files: [shim('0.10.0'), shim('0.9.0')], dirs: [`${cache}/clm/0.10.0`, `${cache}/clm/0.9.0`] })
    expect(await clmCommand(other)).toBe(shim('0.10.0'))
  })

  test("a shim that can't start says so with the reason, not that clm isn't installed", async () => {
    const run = await runClm(fake(REPO), ['view', 'flight'], '/work/repo')
    expect(run.exitCode).toBe(127)
    expect(run.stderr).not.toContain(NOT_INSTALLED)
    expect(run.stderr).toBe(`clm couldn't start: ${SHIM} in /work/repo: spawn ${SHIM} ENOENT`)
  })

  test("a session whose directory is gone says so, not that clm isn't installed", async () => {
    // A spawn in a missing directory fails as ENOENT, as a missing program does.
    const $ = fake({ files: [SHIM] })
    const run = await runClm($, ['view', 'flight'], '/work/repo/.claude/worktrees/gone')
    expect(run.exitCode).toBe(127)
    expect(run.stderr).toBe(sessionDirGone('/work/repo/.claude/worktrees/gone'))
    expect(run.stderr).not.toContain(NOT_INSTALLED)
    const bare = await runClm(fake({}), ['view', 'flight'], '/work/gone')
    expect(bare.stderr).toBe(sessionDirGone('/work/gone'))
  })

  test("a clm on PATH that fails to start for another reason says why, not that it isn't installed", async () => {
    const $ = fake({ files: ['/work/repo/specs/SPEC.md'], dirs: ['/work/repo', '/work/repo/.git'], startError: 'spawn clm ETIMEDOUT' })
    const run = await runClm($, ['view', 'flight'], '/work/repo')
    expect(run.stderr).toBe("clm couldn't start: clm in /work/repo: spawn clm ETIMEDOUT")
  })

  test('a run the engine aborted rejects, rather than reading as a clm that could not start', async () => {
    const $ = fake({ files: [SHIM], dirs: ['/work/repo'], startError: `clm-panel: $.process.run(${SHIM}) aborted` })
    await expect(runClm($, ['view', 'flight'], '/work/repo')).rejects.toThrow(/aborted$/)
  })

  test('from a directory marketplace it runs plugin/bin/clm beside it', async () => {
    expect(await clmCommand(fake({ files: [SHIM] }))).toBe(SHIM)
  })

  test('the shim runs directly, and git is never asked', async () => {
    const $ = fake({ files: [SHIM], answer: (argv: readonly string[]) => (argv[0] === 'git' ? ok('/usr/lib/git-core') : ok('chart')) })
    await runClm($, ['view', 'flight'], '/work/repo')
    expect($.calls).toEqual([{ argv: [SHIM, 'view', 'flight'], cwd: '/work/repo' }])
  })
})
