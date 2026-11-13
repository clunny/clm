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
  tabOf,
  paneBody,
  cachedRun,
  STOP_BUTTONS,
  pressStop,
  stopButtonLabels,
  stopRequested,
  runClm,
  sessionDirGone,
  unlessSuperseded,
  viewKey,
  unfenced,
  viewArgs,
} from '../src/panel.ts'
import { chartLines, modelRuns, paneChart, pressNode } from '../src/inspector.ts'

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
  // req:plugin.panel/view-command-in-chat@56b47f43
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
    for (const name of ['status', 'queue', 'spec', 'flight', 'blockers']) {
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
    expect('text' in answer && answer.text).toContain('status, queue, spec, flight, blockers')
    expect($.calls).toEqual([])
  })
})

describe('views-panel', () => {
  // req:plugin.panel/views-panel@4f33137b
  test('the pane is CLM with Queue, Spec, Flight and Blockers, Queue first', () => {
    expect(PANE_TITLE).toBe('CLM')
    expect(TABS).toEqual(['queue', 'spec', 'flight', 'blockers'])
    expect(FIRST_TAB).toBe('queue')
  })

  test('a kept tab that is none of the four reads as Queue', () => {
    expect(tabOf('capabilities')).toBe('queue')
    expect(tabOf(undefined)).toBe('queue')
    for (const t of TABS) expect(tabOf(t)).toBe(t)
  })

  // req:plugin.panel/queue-tab@0cb57d23
  test('the Queue tab runs no JSON model, and its lines are not pressable and draw no inspector', () => {
    for (const surface of ['terminal', 'desktop'] as const) {
      expect(modelRuns('queue', surface, null, true)).toEqual({})
      expect(modelRuns('queue', surface, 'a', true)).toEqual({})
    }
    expect(viewArgs('queue', 'terminal', 90, true)).toEqual([
      'view', 'queue', '--at', 'origin/main', '--format', 'ansi', '--width', '90',
    ])
    const chart = ansiLines('\x1b[34mqueue\x1b[0m\n  build:git\n')
    // Even given a model, the Queue chart has no pressable line and no inspector.
    const model = ok(JSON.stringify({ nodes: [{ node: 'build:git' }] }))
    const given = chartLines({ tab: 'queue', chart, own: model, selected: 'build:git', unfolded: [] })
    expect(given.inspector).toBeUndefined()
    expect(given.lines.map(l => l.key)).toEqual([undefined, undefined])
    const { lines, inspector } = paneChart({ tab: 'queue', chart, selected: 'a', unfolded: [] })
    expect(inspector).toBeUndefined()
    expect(lines.map(l => l.key)).toEqual([undefined, undefined])
    expect(lines.map(l => l.spans)).toEqual(chart)
  })

  // req:plugin.panel/views-panel@4f33137b
  test('the Queue tab has no SVG form: other surfaces run its text chart', () => {
    expect(viewArgs('queue', 'terminal', 80, true)).toEqual([
      'view', 'queue', '--at', 'origin/main', '--format', 'ansi', '--width', '80',
    ])
    for (const surface of ['desktop', 'vscode', 'mobile'] as const) {
      expect(viewArgs('queue', surface, 80, true)).toEqual(['view', 'queue', '--at', 'origin/main', '--format', 'text'])
      expect(viewArgs('queue', surface, 80)).toEqual(['view', 'queue', '--format', 'text'])
      expect(viewArgs('spec', surface, 80)).toEqual(['view', 'spec', '--format', 'svg'])
    }
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

  // req:plugin.panel/views-panel@4f33137b
  test('a press changes no run the pane holds for this generation, so no clm command runs again', async () => {
    const cache = new Map<string, Promise<Run>>()
    let started = 0
    const start = async (): Promise<Run> => ({ exitCode: 0, stdout: `${++started}`, stderr: '' })
    // A draw's runs: the chart, its model and, with a component selected, the other tab's model.
    const draw = (selected: string | null) => {
      const { own, other } = modelRuns('spec', 'terminal', selected, true)
      const argvs = [viewArgs('spec', 'terminal', 80, true), own!, ...(other ? [other] : [])]
      return Promise.all(argvs.map(argv => cachedRun(cache, viewKey(7, argv), start)))
    }
    await draw('a')
    expect(started).toBe(3)
    // Pressing `b`'s line selects it; pressing `Contract` changes no argument, so the draw is the same.
    await draw(pressNode('a', 'b'))
    await draw('b')
    expect(started).toBe(3)
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
    expect(paneBody('terminal', { exitCode: 1, stdout: 'x', stderr: 'violations: 2' }, 'spec')).toEqual({ error: 'violations: 2' })
    expect(paneBody('terminal', ok('\x1b[34mx\x1b[0m'), 'spec')).toEqual({ ansi: [[{ text: 'x', color: 'blue' }]] })
    expect(paneBody('desktop', ok('<svg/>'), 'spec')).toEqual({ svg: '<svg/>' })
    // The Queue tab's text chart is shown as text where the other tabs draw an SVG.
    expect(paneBody('desktop', ok('queue\n'), 'queue')).toEqual({ text: 'queue\n' })
    expect(paneBody('desktop', ok('<svg/>'), 'spec')).toEqual({ svg: '<svg/>' })
  })

  test('no pictures: no Picture button, no Image, and no png on any tab or surface', () => {
    for (const surface of ['terminal', 'desktop', 'vscode', 'mobile'] as const) {
      for (const tab of ['queue', 'spec', 'flight', 'blockers'] as const) {
        for (const at of [true, false]) expect(viewArgs(tab, surface, 80, at)).not.toContain('png')
      }
    }
    const panel = `${ts.sys.getCurrentDirectory()}/`
    for (const file of ['hooks/register.tsx', 'src/panel.ts', 'src/inspector.ts', 'src/agents.ts']) {
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
  // req:plugin.panel/panel-package@286ccb95
  test("with no clm, it says clm isn't installed, naming the clm plugin", async () => {
    const $ = fake({ files: ['/work/repo/specs/SPEC.md'], dirs: ['/work/repo', '/work/repo/.git'] })
    expect(await clmCommand($)).toBe('clm')
    expect(await command($, 'flight', '/work/repo')).toEqual({ text: NOT_INSTALLED })
    expect(NOT_INSTALLED).toContain('the `clm` plugin')
    // No git either, so no `origin/main`: the checkout's view.
    expect($.calls.at(-1)).toEqual({ argv: ['clm', 'view', 'flight'], cwd: '/work/repo' })
  })

  // req:plugin.panel/panel-package@286ccb95
  test("the manifest names the owner in full and the licence", () => {
    // `npm test` runs in `plugin-panel/`.
    const text = ts.sys.readFile(`${ts.sys.getCurrentDirectory()}/.claude-plugin/plugin.json`)
    const manifest = JSON.parse(text ?? '') as { author: { name: string }; license: string }
    expect(manifest.author.name).toBe('Christopher Lunny')
    expect(manifest.license).toBe('MIT')
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

describe('superseded-draw', () => {
  // req:plugin.panel/superseded-draw@53be288d
  const aborted = new Error(`clm-panel: $.process.run(${SHIM}) aborted`)

  test('a draw whose run the engine aborted ends without throwing and draws nothing, and the newer draw runs again', async () => {
    const cache = new Map<string, Promise<Run>>()
    let starts = 0
    const draw = (outcome: () => Promise<Run>) => async () =>
      (await cachedRun(cache, viewKey(1, ['view', 'flight']), () => (starts++, outcome()))).stdout
    const first = await unlessSuperseded(draw(() => Promise.reject(aborted)), null)
    expect(first).toBeNull()
    const second = await unlessSuperseded(draw(() => Promise.resolve(ok('chart'))), null)
    expect(second).toBe('chart')
    expect(starts).toBe(2)
  })

  test('a failure that is not an abort is not swallowed', async () => {
    await expect(unlessSuperseded(() => Promise.reject(new Error('boom')), null)).rejects.toThrow('boom')
    await expect(unlessSuperseded(() => Promise.reject(new Error('aborted early')), null)).rejects.toThrow('aborted early')
  })

  test('an aborted run of clm or of git is not kept as an answer', async () => {
    const $ = fake({ files: [SHIM], dirs: ['/work/repo'], startError: 'x aborted' })
    expect(await unlessSuperseded(() => runClm($, ['view', 'flight'], '/work/repo'), null)).toBeNull()
    expect(await unlessSuperseded(() => hasOriginMain($, '/work/repo'), null)).toBeNull()
  })

  test('another failure of clm still shows why it could not start', async () => {
    const $ = fake({ files: [SHIM], dirs: ['/work/repo'], startError: 'spawn EACCES' })
    const run = await unlessSuperseded(() => runClm($, ['view', 'flight'], '/work/repo'), null)
    expect(run?.stderr).toBe(`clm couldn't start: ${SHIM} in /work/repo: spawn EACCES`)
  })
})

describe('queue-stop-buttons', () => {
  // req:plugin.panel/queue-stop-buttons@14eb1ea3
  test('Drain and Stop now run clm build stop at the cwd, and a failure answers its stderr', async () => {
    expect(STOP_BUTTONS.map(b => [b.key, b.label])).toEqual([['loop:drain', 'Drain'], ['loop:now', 'Stop now']])
    const $ = fake({ ...REPO, answer: () => ok('') })
    expect(await pressStop($, 'loop:drain', '/work/repo/crates')).toBeUndefined()
    expect(await pressStop($, 'loop:now', '/work/repo')).toBeUndefined()
    expect($.calls).toEqual([
      { argv: [SHIM, 'build', 'stop', '--drain'], cwd: '/work/repo/crates' },
      { argv: [SHIM, 'build', 'stop', '--now'], cwd: '/work/repo' },
    ])
    const failing = fake({ ...REPO, answer: () => ({ exitCode: 2, stdout: '', stderr: 'no state dir\n' }) })
    expect(await pressStop(failing, 'loop:now', '/work/repo')).toBe('no state dir\n')
    expect(await pressStop(failing, 'dismiss:x', '/work/repo')).toBeUndefined()
    expect(failing.calls).toHaveLength(1)
  })

  // req:plugin.panel/queue-stop-buttons@14eb1ea3
  test('a requested stop dims the pressed button with requested', () => {
    const now = ansiLines('queue\n\x1b[33mstop requested: now\x1b[0m\n')
    expect(stopRequested(now)).toBe('now')
    expect(stopButtonLabels('now')).toEqual([
      { key: 'loop:drain', label: 'Drain', dim: false },
      { key: 'loop:now', label: 'Stop now · requested', dim: true },
    ])
    expect(stopButtonLabels(stopRequested(ansiLines('stop requested: drain\n')))[0]).toEqual({
      key: 'loop:drain',
      label: 'Drain · requested',
      dim: true,
    })
    expect(stopRequested(ansiLines('loop held, renewed 1m ago\n'))).toBeUndefined()
    expect(stopButtonLabels(undefined).every(b => !b.dim && !b.label.includes('requested'))).toBe(true)
  })
})
