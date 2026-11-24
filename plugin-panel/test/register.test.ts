// The hooks module as Claude Code drives it: `hooks/register.tsx` registered against a fake `$`,
// its atoms kept in memory, its timers fired by hand, and its pane drawn as a tree of elements.
import { describe, expect, test, vi } from 'vitest'

import type { EngineInterface, On, Process, StepResult, Usage } from 'claude-code'

import { NOT_A_REPO } from '../src/panel.ts'

// The atoms, by `<plugin>/<key>`, and the keys read since the test last cleared them.
const atoms = vi.hoisted(() => ({ values: new Map<string, unknown>(), reads: [] as string[] }))

vi.mock('claude-code', () => {
  type Ref = { key: string; initial: unknown }
  const value = (a: Ref) => (atoms.values.has(a.key) ? atoms.values.get(a.key) : a.initial)
  return {
    atom: (ref: { plugin: string; key: string }, initial: unknown): Ref => ({ key: `${ref.plugin}/${ref.key}`, initial }),
    read: async (_$: unknown, a: Ref) => {
      atoms.reads.push(a.key)
      return value(a)
    },
    update: async (_$: unknown, a: Ref, change: (v: unknown) => unknown) => {
      const next = change(value(a))
      atoms.values.set(a.key, next)
      return next
    },
  }
})

/** A drawn element: its constructor's name, its props and its children. */
type El = { tag: string; props: Record<string, unknown>; children: unknown[] }

const flat = (children: unknown): unknown[] =>
  Array.isArray(children) ? children.flatMap(flat) : children === undefined || children === null || children === false ? [] : [children]
const element =
  (tag: string) =>
  (props: Record<string, unknown>): El => ({ tag, props, children: flat(props.children) })
const ELEMENTS = { Box: element('Box'), Text: element('Text'), Button: element('Button'), Code: element('Code'), Svg: element('Svg') }

// The JSX factory the module compiles against.
Object.assign(globalThis, {
  h: (tag: (props: Record<string, unknown>) => unknown, props: Record<string, unknown> | null, ...children: unknown[]) =>
    tag({ ...(props ?? {}), children }),
  Fragment: (props: { children?: unknown }) => element('Fragment')(props),
})

const isEl = (n: unknown): n is El => typeof n === 'object' && n !== null && 'tag' in n

/** An element's text: its strings, a labelled button as `[label]`, a row's parts spaced. */
function textOf(n: unknown): string {
  if (typeof n === 'string') return n
  if (!isEl(n)) return ''
  const label = n.tag === 'Button' && typeof n.props.label === 'string' ? `[${n.props.label}]` : ''
  const parts = n.children.map(textOf)
  return label + parts.join(n.tag === 'Box' && n.props.flexDirection === 'row' ? ' ' : '')
}

/** The pane's lines: each child of a column on a line of its own. */
function linesOf(n: unknown): string[] {
  if (isEl(n) && n.tag === 'Box' && n.props.flexDirection === 'column') return n.children.flatMap(linesOf)
  return [textOf(n)]
}

function buttons(n: unknown): El[] {
  if (!isEl(n)) return []
  return [...(n.tag === 'Button' ? [n] : []), ...n.children.flatMap(buttons)]
}

const PANEL = '/plugins/clm/plugin-panel'
const SHIM = '/plugins/clm/plugin/bin/clm'
const REPO = '/work/repo'

const ok = (stdout: string): Process => ({ exitCode: 0, stdout, stderr: '' })

/** A deferred answer, settled by the test. */
function deferred<T>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)))
  return { promise, resolve, reject }
}

type Timer = { ms: number; run: () => void; cancelled: boolean }
type Hook = (...args: never[]) => unknown

/** Claude Code as the module sees it, with the hooks it registered. */
function world() {
  const w = {
    cwd: REPO,
    now: 0,
    commit: 'abc1234',
    calls: [] as string[],
    timers: [] as Timer[],
    panes: [] as { id: string; isPlaced: boolean }[],
    opens: 0,
    openFails: false,
    agents: async (): Promise<{ id: string; status: string }[]> => [],
    stop: ok(''),
    fetch: async (): Promise<Process> => ok(''),
    hooks: [] as { event: string; match?: Record<string, unknown>; hook: Hook }[],
  }
  const files = new Set([SHIM, `${REPO}/specs/SPEC.md`])
  const dirs = new Set([REPO, `${REPO}/.git`])
  const $ = {
    plugin: { root: PANEL },
    process: {
      async run(argv: readonly string[]) {
        const line = argv.join(' ')
        w.calls.push(line)
        if (line === 'git fetch --quiet origin') return w.fetch()
        if (argv[0] === 'git') return ok(`${w.commit}\n`)
        if (argv[1] === 'build') return w.stop
        if (argv[1] === 'machine') return ok('')
        if (argv[2] === 'queue') {
          return ok(`origin/main ${w.commit}\nloop not held\n\x1b[34mrunning\x1b[0m\n  build:git    P5  \x1b[33mrunning\x1b[0m\nnext up\n  none\n`)
        }
        return ok(`origin/main ${w.commit}\n${argv[2]} chart\n`)
      },
    },
    fs: {
      async stat(path: string) {
        if (files.has(path)) return { kind: 'file' as const }
        if (dirs.has(path)) return { kind: 'dir' as const }
        throw new Error(`ENOENT ${path}`)
      },
      async list(): Promise<{ name: string; kind: string }[]> {
        throw new Error('ENOENT')
      },
    },
    session: { cwd: async () => w.cwd },
    clock: {
      now: async () => w.now,
      every(ms: number, run: () => void) {
        const t = { ms, run, cancelled: false }
        w.timers.push(t)
        return { cancel: () => void (t.cancelled = true) }
      },
    },
    agent: { list: () => w.agents() },
    ui: {
      panes: async () => w.panes.map(p => ({ ...p })),
      async open(pane: { id: string; title: string }) {
        if (w.openFails) throw new Error('no room')
        w.opens++
        w.panes.push({ id: pane.id, isPlaced: true })
      },
      resolve: (() => ELEMENTS) as unknown as EngineInterface['ui']['resolve'],
    },
  } as EngineInterface
  return { w, $ }
}

/** Lets every promise the module started settle: no timer runs, so the chain ends. */
async function flush() {
  for (let i = 0; i < 500; i++) await Promise.resolve()
}

async function setup() {
  vi.resetModules()
  // A fresh session: no atom kept, no module state.
  atoms.values.clear()
  atoms.reads.length = 0
  const { register } = await import('../hooks/register.tsx')
  const { w, $ } = world()
  const on = (event: string, a: Hook | Record<string, unknown>, b?: Hook) =>
    w.hooks.push(b ? { event, match: a as Record<string, unknown>, hook: b } : { event, hook: a as Hook })
  register(on as unknown as On)
  const hook = (event: string, match?: (m: Record<string, unknown>) => boolean) => {
    const found = w.hooks.find(h => h.event === event && (!match || (h.match !== undefined && match(h.match))))
    if (!found) throw new Error(`no ${event} hook`)
    return found.hook as unknown as (...args: unknown[]) => Promise<unknown>
  }
  const pass = <T>(value: T) => async () => value

  const harness = {
    w,
    $,
    /** `/clm:view` with `args`. */
    async view(args: string) {
      const answer = await hook('command.run')($, { args })
      await flush()
      return answer
    },
    /** The pane drawn `columns` wide on `surface`: its lines and its buttons. */
    async draw(surface = 'terminal', columns = 80) {
      await flush()
      const root = await hook('ui.render', m => m.component === 'Pane')($, { surface, props: { bodyColumns: columns } })
      await flush()
      return { lines: linesOf(root), buttons: buttons(root) }
    },
    /** Presses the button labelled `label`, or whose label text is `label`. */
    async press(label: string) {
      const { buttons: all } = await harness.draw()
      const b = all.find(x => x.props.label === label || textOf(x) === label)
      if (!b) throw new Error(`no button ${label}`)
      ;(b.props.onPress as () => void)()
      await flush()
    },
    /** Fires the timers of period `ms`, at `now`. */
    async fire(ms: number, now = w.now) {
      w.now = now
      for (const t of w.timers.filter(t => t.ms === ms && !t.cancelled)) t.run()
      await flush()
    },
    async turnEnds(now = w.now, e: Record<string, unknown> = { reason: 'answer' }) {
      w.now = now
      await hook('turn.complete')($, e, pass(undefined))
      await flush()
    },
    async skill(e: { skill: string; pluginName?: string; text: string }) {
      const prompt = { text: e.text }
      const answer = await hook('skill.prompt')($, e, pass(prompt))
      await flush()
      return { prompt, answer }
    },
    async sessionStart() {
      await hook('session.start')($, {}, pass(undefined))
      await flush()
    },
    async spawn(agentId: string, description: string, prompt = 'loop run build:git\nGo.', at = w.now) {
      w.now = at
      await hook('agent.spawn')($, { prompt, description, subagentType: 'general-purpose' }, pass({ agentId, model: 'claude-opus-5-5' }))
      await flush()
    },
    async step(agentId: string, usage: Usage) {
      const stepHook = hook('turn.step') as unknown as (
        $: EngineInterface,
        e: unknown,
        next: () => AsyncGenerator<never, StepResult, undefined>,
      ) => AsyncGenerator<never, StepResult, undefined>
      const gen = stepHook($, { agentId, model: 'claude-opus-5-5' }, async function* () {
        return { usage }
      })
      while (!(await gen.next()).done);
      await flush()
    },
    /** The `clm view` runs so far. */
    views: () => w.calls.filter(c => c.startsWith(`${SHIM} view `)),
  }
  return harness
}

const tokens = (input: number): Usage => ({
  input_tokens: input,
  output_tokens: 0,
  cache_read_input_tokens: 0,
  cache_creation_input_tokens: 0,
})


describe('views-panel', () => {
  // req:plugin.panel/views-panel@76d63404
  test('/clm:view with no argument opens the pane on Queue and answers Opened the CLM panel.', async () => {
    const p = await setup()
    atoms.values.set('clm-panel/tab', 'spec')
    expect(await p.view('')).toEqual({ text: 'Opened the CLM panel.' })
    expect(p.w.panes).toEqual([{ id: 'clm', isPlaced: true }])
    const { lines, buttons } = await p.draw('terminal', 80)
    expect(p.views()).toContain(`${SHIM} view queue --at origin/main --format ansi --width 80`)
    expect(buttons.find(b => b.props.label === 'Queue')?.props.variant).toBe('primary')
    expect(lines).toContain('  build:git    P5  running')
    // Outside a CLM repository it says so and opens nothing.
    const out = await setup()
    out.w.cwd = '/elsewhere'
    expect(await out.view('')).toEqual({ text: NOT_A_REPO })
    expect(out.w.panes).toEqual([])
  })

  // req:plugin.panel/views-panel@76d63404
  test('the pane redraws on open, on a tab change, when a turn ends and every 30 seconds while placed', async () => {
    const p = await setup()
    const queue = () => p.views().filter(c => c.includes(' view queue ')).length
    const fetches = () => p.w.calls.filter(c => c === 'git fetch --quiet origin').length
    await p.view('')
    await p.draw()
    await p.draw()
    expect(queue()).toBe(1)
    // A turn ends, the timer fires 30 seconds later, and three turns end within the next minute.
    await p.turnEnds(0)
    await p.draw()
    expect(queue()).toBe(2)
    await p.fire(30_000, 30_000)
    await p.draw()
    expect(queue()).toBe(3)
    for (const at of [40_000, 55_000, 70_000]) {
      await p.turnEnds(at)
      await p.draw()
    }
    expect(queue()).toBe(6)
    expect(fetches()).toBe(1)
    // A tab change runs `clm view` again, even for a tab shown before.
    await p.press('Spec')
    await p.draw()
    await p.press('Queue')
    await p.draw()
    expect(queue()).toBe(7)
    expect(p.views().filter(c => c.includes(' view spec ')).length).toBeGreaterThan(0)
    // Not placed, the timer redraws nothing.
    p.w.panes[0]!.isPlaced = false
    await p.fire(30_000, 100_000)
    await p.draw()
    expect(queue()).toBe(7)
    // Placed again, past 2 minutes, it fetches again.
    p.w.panes[0]!.isPlaced = true
    await p.fire(30_000, 120_000)
    expect(fetches()).toBe(2)
  })

  // req:plugin.panel/views-panel@76d63404
  test('a redraw that fetches runs the views again once the fetch ends, and a cut-short fetch leaves the failed line', async () => {
    const p = await setup()
    const first = deferred<Process>()
    p.w.fetch = () => first.promise
    await p.view('')
    expect((await p.draw()).lines).toContain('origin/main abc1234')
    // The fetch moves origin/main; once it ends the pane runs `clm view` again.
    p.w.commit = 'def5678'
    first.resolve(ok(''))
    const before = p.views().length
    const after = await p.draw()
    expect(p.views().length).toBe(before + 1)
    expect(after.lines).toContain('origin/main def5678')
    // A failed fetch draws its line; one cut short later leaves it, aged from the failed one's start.
    p.w.fetch = async () => ({ exitCode: 128, stdout: '', stderr: 'fatal: unable to access x\nmore\n' })
    await p.turnEnds(120_000)
    expect((await p.draw()).lines).toContain('⚠ fetch failed 0m ago: fatal: unable to access x')
    p.w.fetch = () => Promise.reject(new Error('clm-panel: $.process.run(git) aborted'))
    await p.turnEnds(240_000)
    expect((await p.draw()).lines).toContain('⚠ fetch failed 2m ago: fatal: unable to access x')
    // One that succeeds clears it.
    p.w.fetch = async () => ok('')
    await p.turnEnds(360_000)
    expect((await p.draw()).lines.some(l => l.startsWith('⚠ fetch failed'))).toBe(false)
  })
})

describe('build-opens-panel', () => {
  // req:plugin.panel/build-opens-panel@d27a61fc
  test("the build skill's prompt opens the closed pane on Queue and comes back unchanged", async () => {
    for (const skill of [{ skill: 'clm:build' }, { skill: 'build', pluginName: 'clm' }]) {
      const p = await setup()
      atoms.values.set('clm-panel/tab', 'flight')
      const { prompt, answer } = await p.skill({ ...skill, text: 'Build it.' })
      expect(answer).toBe(prompt)
      expect(prompt).toEqual({ text: 'Build it.' })
      expect(p.w.panes).toEqual([{ id: 'clm', isPlaced: true }])
      await p.draw()
      expect(p.views()).toContain(`${SHIM} view queue --at origin/main --format ansi --width 80`)
      // Redrawing and sampling from then on.
      expect(p.w.timers.filter(t => !t.cancelled).map(t => t.ms).sort()).toEqual([10_000, 30_000])
    }
  })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('an open pane keeps its tab; outside a CLM repository or for another skill nothing opens', async () => {
    const open = await setup()
    atoms.values.set('clm-panel/tab', 'spec')
    open.w.panes = [{ id: 'clm', isPlaced: false }]
    const kept = await open.skill({ skill: 'clm:build', text: 'Build it.' })
    expect(kept.answer).toBe(kept.prompt)
    expect(open.w.opens).toBe(0)
    expect(atoms.values.get('clm-panel/tab')).toBe('spec')

    const outside = await setup()
    outside.w.cwd = '/elsewhere'
    const out = await outside.skill({ skill: 'clm:build', text: 'Build it.' })
    expect(out.answer).toBe(out.prompt)
    expect(outside.w.opens).toBe(0)

    for (const other of [{ skill: 'clm:propose' }, { skill: 'build' }, { skill: 'build', pluginName: 'other' }]) {
      const p = await setup()
      const { prompt, answer } = await p.skill({ ...other, text: 'Go.' })
      expect(answer).toBe(prompt)
      expect(p.w.opens).toBe(0)
      expect(p.w.calls).toEqual([])
    }
  })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('a failure while opening leaves the prompt as it came', async () => {
    const p = await setup()
    p.w.openFails = true
    const { prompt, answer } = await p.skill({ skill: 'clm:build', text: 'Build it.' })
    expect(answer).toBe(prompt)
    expect(prompt).toEqual({ text: 'Build it.' })
    expect(p.w.panes).toEqual([])
  })
})

describe('queue-stop-buttons', () => {
  // req:plugin.panel/queue-stop-buttons@03474fbd
  test('Drain runs clm build stop, then the Queue view again; a failure shows until the next press, redraw or tab change', async () => {
    const p = await setup()
    await p.view('')
    await p.draw()
    const before = p.views().length
    await p.press('Drain')
    const drawn = await p.draw()
    const stop = p.w.calls.indexOf(`${SHIM} build stop --drain`)
    expect(stop).toBeGreaterThanOrEqual(0)
    expect(p.views().length).toBe(before + 1)
    expect(p.w.calls.lastIndexOf(`${SHIM} view queue --at origin/main --format ansi --width 80`)).toBeGreaterThan(stop)
    expect(drawn.lines).toContain('  build:git    P5  running')

    // A failure's stderr shows above the chart until the next press.
    p.w.stop = { exitCode: 2, stdout: '', stderr: 'no state dir' }
    await p.press('Stop now')
    expect((await p.draw()).lines).toContain('no state dir')
    p.w.stop = ok('')
    await p.press('Drain')
    expect((await p.draw()).lines).not.toContain('no state dir')
    // Or the next redraw.
    p.w.stop = { exitCode: 2, stdout: '', stderr: 'no state dir' }
    await p.press('Stop now')
    expect((await p.draw()).lines).toContain('no state dir')
    await p.turnEnds()
    expect((await p.draw()).lines).not.toContain('no state dir')
    // Or a tab change.
    await p.press('Stop now')
    expect((await p.draw()).lines).toContain('no state dir')
    await p.press('Spec')
    await p.press('Queue')
    expect((await p.draw()).lines).not.toContain('no state dir')
  })
})

describe('build-row-cost', () => {
  // req:plugin.panel/build-row-cost@8fdc23db
  test('the rows tick each second without clm running, on the Queue tab only', async () => {
    const p = await setup()
    await p.sessionStart()
    await p.view('')
    await p.spawn('b1', 'build git', 'loop run build:git\nGo.', 0)
    expect((await p.draw()).lines.join('\n')).toContain('0:00')
    const views = p.views().length
    await p.fire(1000, 1000)
    expect((await p.draw()).lines.join('\n')).toContain('0:01')
    await p.fire(1000, 2000)
    expect((await p.draw()).lines.join('\n')).toContain('0:02')
    expect(p.views().length).toBe(views)
    // Another tab draws no rows and doesn't read the time.
    await p.press('Spec')
    await p.draw()
    atoms.reads.length = 0
    await p.fire(1000, 3000)
    const spec = await p.draw()
    expect(spec.lines.join('\n')).not.toContain('build git')
    expect(atoms.reads).not.toContain('clm-panel/now')
  })
})

describe('build-row-lifecycle', () => {
  // req:plugin.panel/build-row-lifecycle@015a4b69
  test('a failing agent list at session start or at the 30-second redraw leaves the runs', async () => {
    const p = await setup()
    await p.view('')
    await p.spawn('b1', 'build git', 'loop run build:git\nGo.', 0)
    p.w.agents = async () => [{ id: 'b1', status: 'waiting' }]
    await p.turnEnds(5_000, { agentId: 'b1', reason: 'answer' })
    const waiting = (lines: string[]) => lines.some(l => /● build git · waiting$/.test(l))
    expect(waiting((await p.draw()).lines)).toBe(true)
    p.w.agents = () => Promise.reject(new Error('agents unavailable'))
    await p.sessionStart()
    expect(waiting((await p.draw()).lines)).toBe(true)
    await p.fire(30_000, 30_000)
    expect(waiting((await p.draw()).lines)).toBe(true)
  })

  // req:plugin.panel/build-row-lifecycle@015a4b69
  test("at session start a running run Claude Code no longer lists is dropped, and the totals don't count it", async () => {
    for (const oneWaits of [false, true]) {
      const p = await setup()
      await p.view('')
      for (const [id, description, input] of [['a1', 'one', 1_000], ['a2', 'two', 2_000], ['a3', 'three', 3_000]] as const) {
        await p.spawn(id, description, 'Look.', 0)
        await p.step(id, tokens(input))
      }
      p.w.agents = async () => [{ id: 'a1', status: oneWaits ? 'waiting' : 'running' }]
      await p.turnEnds(1_000, { agentId: 'a2', reason: 'answer' })
      if (oneWaits) await p.turnEnds(1_000, { agentId: 'a1', reason: 'answer' })
      expect((await p.draw()).lines.at(-1)).toMatch(/· 6k tokens ·/)
      // After the reload Claude Code lists only `three`'s agent.
      p.w.agents = async () => [{ id: 'a3', status: 'running' }]
      await p.sessionStart()
      const { lines } = await p.draw()
      expect(lines.at(-1)).toMatch(/· 5k tokens ·/)
      expect(lines.some(l => /● one/.test(l))).toBe(false)
      expect(lines.some(l => /● three/.test(l))).toBe(true)
    }
  })
})
