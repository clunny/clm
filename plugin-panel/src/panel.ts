// The CLM panel's rules: which `clm` to run and how, what `/clm:view` answers, ANSI escapes
// to coloured runs, and when to fetch. They take only the few `$` calls they use, so the
// tests run them against a fake; `hooks/register.tsx` wires them to the real `$`.

/** What `/clm:view <name>` accepts: `status` runs `clm status`, the rest `clm view <name>`. */
export const NAMES = ['status', 'queue', 'spec', 'flight', 'blockers'] as const

/** The pane's tabs, in order; it opens on `queue`. Each shows `clm view <tab>`. */
export const TABS = ['queue', 'spec', 'flight', 'blockers'] as const
export type ViewTab = (typeof TABS)[number]
export const FIRST_TAB: ViewTab = 'queue'
/** A tab kept across a reload, read as one of the four; any other reads as Queue (`views-panel`). */
export const tabOf = (kept: unknown): ViewTab => TABS.find(t => t === kept) ?? FIRST_TAB

export const PANE_ID = 'clm'
export const PANE_TITLE = 'CLM'
/** How often an open pane redraws, and how often at most it fetches `origin`. */
export const REDRAW_MS = 30_000
export const FETCH_MS = 120_000

export type Surface = 'terminal' | 'desktop' | 'vscode' | 'mobile'
/** A finished process. */
export type Run = { exitCode: number; stdout: string; stderr: string }

/** The calls on `$` the rules use. */
export type Host = {
  plugin: { root: string }
  process: {
    run(argv: readonly string[], init?: { cwd?: string }): Promise<Run>
  }
  fs: {
    stat(path: string): Promise<{ kind: 'file' | 'dir' | 'other' }>
    list(path?: string): Promise<readonly { name: string; kind: string }[]>
  }
}

export const NOT_INSTALLED =
  "clm isn't installed: install the `clm` plugin from the `clm` marketplace " +
  '(`/plugin install clm@clm`), or put `clm` on PATH.'

/** What the panel says when the session's directory no longer exists, such as a removed worktree. */
export const sessionDirGone = (cwd: string) =>
  `This session's directory no longer exists: ${cwd}. Move the session back into the repository (\`cd\` there) to see CLM's views.`

/** A start error that means the program isn't there. */
const NOT_FOUND = /ENOENT|not found|no such file/i

export const NOT_A_REPO =
  "This directory isn't a CLM repository: no `specs/SPEC.md` here or above it, up to the git root."

function exists($: Host, path: string, kind?: 'file'): Promise<boolean> {
  return $.fs.stat(path).then(
    s => kind === undefined || s.kind === kind,
    () => false,
  )
}

/** A path to look things up by: no trailing separator but the root's. */
const slashed = (path: string) => path.replace(/\/+$/, '') || '/'

/** The directory holding `dir`; the root is its own. */
const parent = (dir: string) => slashed(dir.replace(/\/[^/]*$/, '') || '/')

const join = (dir: string, rest: string) => (dir.endsWith('/') ? dir + rest : `${dir}/${rest}`)

/**
 * The CLM repository holding `cwd`, as `session-start` finds it: the first of `cwd` and its
 * ancestors with `specs/SPEC.md`, looking no higher than the first that holds `.git`, nor
 * than the root (`/`).
 */
export async function clmRoot($: Host, cwd: string): Promise<string | undefined> {
  for (let dir = slashed(cwd); ; dir = parent(dir)) {
    if (await exists($, join(dir, 'specs/SPEC.md'), 'file')) return dir
    if (await exists($, join(dir, '.git'))) return undefined
    if (parent(dir) === dir) return undefined
  }
}

const versionKey = (v: string) => v.split(/[.-]/).map(p => p.padStart(8, '0')).join('.')

/**
 * The `clm` to run: the `clm` plugin's `bin/clm` shim, so the panel gets the same binary and
 * `version-skew` handling as the Bash tool, else `clm` on PATH. Installed from a directory
 * marketplace the two plugins sit side by side (`plugin/`, `plugin-panel/`); from the plugin
 * cache they sit at `<marketplace>/clm/<version>` and `<marketplace>/clm-panel/<version>`,
 * and the panel prefers the `clm` of its own version.
 */
export async function clmCommand($: Host): Promise<string> {
  const root = slashed($.plugin.root)
  const sibling = join(parent(root), 'plugin/bin/clm')
  if (await exists($, sibling, 'file')) return sibling
  const own = root.slice(root.lastIndexOf('/') + 1)
  const versions = join(parent(parent(root)), 'clm')
  const listed = await $.fs.list(versions).catch(() => [])
  const names = listed
    .filter(e => e.kind === 'dir')
    .map(e => e.name)
    .sort((a, b) => (a === own ? -1 : b === own ? 1 : versionKey(b).localeCompare(versionKey(a))))
  for (const name of names) {
    const shim = `${versions}/${name}/bin/clm`
    if (await exists($, shim, 'file')) return shim
  }
  return 'clm'
}

/** A `$` call the engine cut short, such as a draw's run once a newer draw replaced it. */
const ABORTED = /\baborted$/

/**
 * A pane draw's answer, or `none` when the engine aborted one of its `$` calls because a newer
 * draw replaced it: the superseded draw ends quietly and the newer one draws. Any other failure
 * is thrown.
 */
export async function unlessSuperseded<T, N>(draw: () => Promise<T>, none: N): Promise<T | N> {
  try {
    return await draw()
  } catch (e) {
    if (ABORTED.test(e instanceof Error ? e.message : String(e))) return none
    throw e
  }
}

/**
 * Runs `clm` with `args` at `cwd`; a `clm` that can't start reads as a failure saying so: not
 * installed when no shim was found and there is no `clm` on PATH, else why it didn't start. A
 * run the engine aborted rejects, so no cache keeps it as `clm`'s failure.
 */
export async function runClm($: Host, args: readonly string[], cwd: string): Promise<Run> {
  const clm = await clmCommand($)
  try {
    const run = await $.process.run([clm, ...args], { cwd })
    // A failure always says something: stderr, else stdout, else the exit code.
    if (run.exitCode !== 0 && run.stderr.trim() === '') {
      return { ...run, stderr: run.stdout.trim() || `clm exited ${run.exitCode}` }
    }
    return run
  } catch (e) {
    // A missing `cwd` fails the spawn as ENOENT too, so it's checked before blaming `clm`.
    if (!(await exists($, cwd))) return { exitCode: 127, stdout: '', stderr: sessionDirGone(cwd) }
    const why = e instanceof Error ? e.message : String(e)
    if (ABORTED.test(why)) throw e
    if (clm === 'clm' && NOT_FOUND.test(why)) return { exitCode: 127, stdout: '', stderr: NOT_INSTALLED }
    return { exitCode: 127, stdout: '', stderr: `clm couldn't start: ${clm} in ${cwd}: ${why}` }
  }
}

/** Whether `origin/main` exists, so the pane's views describe what has landed. */
export const ORIGIN_MAIN_ARGV = ['git', 'rev-parse', '--verify', '-q', 'refs/remotes/origin/main'] as const

/**
 * Whether `refs/remotes/origin/main` exists at `cwd`; a git that can't start reads as no. A run
 * the engine aborted rejects, so no one keeps it as the answer.
 */
export async function hasOriginMain($: Host, cwd: string): Promise<boolean> {
  try {
    return (await $.process.run(ORIGIN_MAIN_ARGV, { cwd })).exitCode === 0
  } catch (e) {
    if (ABORTED.test(e instanceof Error ? e.message : String(e))) throw e
    return false
  }
}

/**
 * `clm view <tab>`, of `origin/main` when `at` (the ref exists), else of the checkout: every
 * `clm view` the pane runs starts so.
 */
export const viewOf = (tab: ViewTab, at: boolean): string[] => ['view', tab, ...(at ? ['--at', 'origin/main'] : [])]

/** `--width` takes 1 to 65535; a pane not yet measured gets clm's default. */
export const widthOf = (columns: number): number =>
  Number.isFinite(columns) ? Math.min(65535, Math.max(1, Math.floor(columns))) : 100

/**
 * The `clm view` arguments for a tab on a surface `columns` wide: an SVG, or ANSI on the
 * terminal; the Queue tab has no SVG form, so other surfaces run its text chart.
 */
export function viewArgs(tab: ViewTab, surface: Surface, columns: number, at = false): string[] {
  const view = viewOf(tab, at)
  if (surface !== 'terminal') return [...view, '--format', tab === 'queue' ? 'text' : 'svg']
  return [...view, '--format', 'ansi', '--width', String(widthOf(columns))]
}

/** What the pane draws under its tabs, from its `clm` run: `clm`'s stderr first. */
export type PaneBody = { error: string } | { ansi: Span[][] } | { svg: string } | { text: string }

export function paneBody(surface: Surface, run: Run, tab: ViewTab): PaneBody {
  if (run.exitCode !== 0) return { error: run.stderr }
  if (surface === 'terminal') return { ansi: ansiLines(run.stdout) }
  return tab === 'queue' ? { text: run.stdout } : { svg: run.stdout }
}

/** The pane's cache key for one `clm` run: its redraw generation and arguments. */
export const viewKey = (generation: number, args: readonly string[]): string => `${generation} ${args.join(' ')}`

/**
 * The pane's `clm` run for `key`, started once: a draw that changes nothing the run depends on,
 * such as a build row's tick, reuses it. A failed start is forgotten, so the next draw retries,
 * and a draw that was waiting on it starts its own run. A run of a newer generation drops the
 * older ones, whose outputs no draw reads again.
 */
export function cachedRun<T = Run>(cache: Map<string, Promise<T>>, key: string, start: () => Promise<T>): Promise<T> {
  const held = cache.get(key)
  // `pending.catch` below, attached first, has forgotten the failed run by the time this retries.
  if (held) return held.catch(() => cachedRun(cache, key, start))
  const generation = key.slice(0, key.indexOf(' '))
  for (const old of cache.keys()) if (!old.startsWith(`${generation} `)) cache.delete(old)
  // A resize within one generation adds a width each time.
  if (cache.size > 16) cache.clear()
  const pending = start()
  cache.set(key, pending)
  pending.catch(() => cache.delete(key))
  return pending
}

export type Answer = { text: string } | { open: true }

// It starts on a line of its own: the transcript heads a command's answer with the plugin's name.
const fenced = (text: string) => '\n```\n' + text.replace(/\n?$/, '\n') + '```'

/** The text inside the code block `command` answers with, or undefined for any other answer. */
export function unfenced(text: string): string | undefined {
  return /```\n([\s\S]*?)\n?```\s*$/.exec(text)?.[1]
}

/**
 * What `/clm:view <args>` answers: `{ open }` for the pane, else the text to show; a view is
 * of `origin/main` while that ref exists.
 */
export async function command($: Host, args: string, cwd: string): Promise<Answer> {
  const name = args.trim()
  if (name !== '' && !(NAMES as readonly string[]).includes(name)) {
    return { text: `/clm:view takes one of: ${NAMES.join(', ')}; with none, it opens the panel.` }
  }
  if ((await clmRoot($, cwd)) === undefined) return { text: NOT_A_REPO }
  if (name === '') return { open: true }
  const argv = name === 'status' ? ['status'] : viewOf(name as ViewTab, await hasOriginMain($, cwd))
  const run = await runClm($, argv, cwd)
  return { text: run.exitCode === 0 ? fenced(run.stdout) : run.stderr }
}

/**
 * Whether a skill's prompt being expanded opens the pane: the `clm` plugin's `build` skill, in
 * a CLM repository, with the panel's pane not listed (placed or not) (`build-opens-panel`).
 */
export function opensForSkill(i: {
  skill: string
  pluginName?: string
  panes: readonly { id: string; isPlaced?: boolean }[]
  inRepo: boolean
}): boolean {
  const isBuild = i.skill === 'clm:build' || (i.skill === 'build' && i.pluginName === 'clm')
  return isBuild && i.inRepo && !i.panes.some(p => p.id === PANE_ID)
}

// `colour-meanings`, named as `ansiLines` names `clm view`'s ANSI colours.
export const AMBER = 'yellow'
export const GREEN = 'green'
export const ROSE = 'red'
export const INDIGO = 'blue'

/**
 * A run of text in one style. With `button`, it is a button labelled `text` whose `key` is
 * `button` (a build row's `Dismiss`), drawn last on its line.
 */
export type Span = { text: string; color?: string; bold?: boolean; dim?: boolean; button?: string }

const COLORS = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white']

/** ANSI text as lines of styled runs: SGR escapes set the style, other escapes are dropped. */
export function ansiLines(text: string): Span[][] {
  let style: Omit<Span, 'text'> = {}
  return text
    .replace(/\n$/, '')
    .split('\n')
    .map(line => {
      const spans: Span[] = []
      // eslint-disable-next-line no-control-regex
      // CSI sequences, and OSC ones (a hyperlink's ends), whose text between stays.
      for (const part of line.split(/(\x1b\[[0-9;]*[A-Za-z]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\))/)) {
        const sgr = /^\x1b\[([0-9;]*)m$/.exec(part)
        if (sgr) {
          style = applySgr(style, sgr[1] ?? '')
        } else if (part !== '' && !part.startsWith('\x1b')) {
          spans.push({ text: part, ...style })
        }
      }
      return spans
    })
}

function applySgr(style: Omit<Span, 'text'>, params: string): Omit<Span, 'text'> {
  let next = { ...style }
  const codes = (params === '' ? '0' : params).split(';').map(Number)
  for (let i = 0; i < codes.length; i++) {
    const code = codes[i] ?? 0
    // 38/48 take a 256-colour index (5;n) or an RGB triple (2;r;g;b): skipped, not drawn.
    if (code === 38 || code === 48) i += codes[i + 1] === 5 ? 2 : 4
    else if (code === 0) next = {}
    else if (code === 1) next.bold = true
    else if (code === 2) next.dim = true
    else if (code === 22) next = { ...(next.color && { color: next.color }) }
    else if (code === 39) delete next.color
    else if (code >= 30 && code <= 37) next.color = COLORS[code - 30]
    else if (code >= 90 && code <= 97) next.color = `${COLORS[code - 90]}Bright`
  }
  return next
}

/**
 * Whether a redraw at `now` also fetches `origin`: at most once every `FETCH_MS`, the last
 * fetch having started at `lastFetch`.
 */
export function shouldFetch(lastFetch: number | undefined, now: number): boolean {
  return lastFetch === undefined || now - lastFetch >= FETCH_MS
}

export const FETCH_ARGV = ['git', 'fetch', '--quiet', 'origin'] as const

/** A finished fetch: when it started, how it exited, and its stderr; a git that couldn't start exits -1. */
export type Fetched = { startedAt: number; exitCode: number; stderr: string }

/**
 * A redraw's fetch: starts `git fetch --quiet origin` at `cwd` when one is due. It answers at
 * once with when the last fetch started, so a redraw that overlaps this one sees it started,
 * and with the fetch while one runs, which gives how it went, or nothing when the engine cut it
 * short. A failed fetch still counts, so an offline session doesn't retry on every redraw.
 */
export function fetchIfDue(
  $: Host,
  cwd: string,
  now: number,
  lastFetch: number | undefined,
): { lastFetch: number | undefined; fetching?: Promise<Fetched | undefined> } {
  if (!shouldFetch(lastFetch, now)) return { lastFetch }
  const fetching = $.process.run(FETCH_ARGV, { cwd }).then(
    run => ({ startedAt: now, exitCode: run.exitCode, stderr: run.stderr }),
    e => {
      const why = e instanceof Error ? e.message : String(e)
      return ABORTED.test(why) ? undefined : { startedAt: now, exitCode: -1, stderr: why }
    },
  )
  return { lastFetch: now, fetching }
}

/** How long ago, as `view-at-rev` writes it: whole minutes under an hour (`0m` under one), else whole hours. */
export const fetchAge = (ms: number): string => {
  const minutes = Math.floor(Math.max(0, ms) / 60_000)
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`
}

/**
 * The amber line drawn above the chart while the latest fetch failed: `⚠ fetch failed <age>
 * ago: ` and the first line of git's stderr, `<age>` since it started; none once one succeeds.
 */
export function fetchWarning(latest: Fetched | undefined, now: number): Span[] | undefined {
  if (latest === undefined || latest.exitCode === 0) return undefined
  const why = latest.stderr.trim().split('\n', 1)[0] || `git exited ${latest.exitCode}`
  return [{ text: `⚠ fetch failed ${fetchAge(now - latest.startedAt)} ago: ${why}`, color: AMBER }]
}
