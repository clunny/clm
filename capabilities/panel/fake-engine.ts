// A fake of Claude Code's plugin engine, as much of it as the panel's module touches. It reads
// a plugin directory's `hooks/hooks.json`, imports each module its `modules` names, calls the
// module's `register` and then plays the engine: it answers a slash command, keeps the
// module's atoms, draws the pane on every atom change, runs the module's timers on a clock
// the test advances, and runs real processes. The module imports `claude-code`, which the
// package's vitest config aliases to this file.
import { execFile } from 'node:child_process'
import { readFile, readdir, stat } from 'node:fs/promises'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

/** An element the module drew, as the engine's `h` makes it. */
export type Node = { type: unknown; props: Record<string, any>; children: unknown[] }

type AtomKey = { plugin: string; key: string }
type Atom<T> = { id: AtomKey; initial: T }
type Store = Map<string, unknown>

const slot = (a: Atom<unknown>) => `${a.id.plugin}/${a.id.key}`

export function atom<T>(id: AtomKey, initial: T, _options?: unknown): Atom<T> {
  return { id, initial }
}

export async function read<T>($: any, a: Atom<T>): Promise<T> {
  const store = $.__store as Store
  return (store.has(slot(a)) ? store.get(slot(a)) : a.initial) as T
}

export async function update<T>($: any, a: Atom<T>, f: (was: T) => T): Promise<void> {
  const store = $.__store as Store
  store.set(slot(a), f(await read($, a)))
  $.__changed()
}

type Handler = (...args: any[]) => any
type Hook = { event: string; filter: Record<string, any>; handler: Handler }

const matches = (filter: Record<string, any>, e: Record<string, any>): boolean =>
  Object.entries(filter).every(([k, v]) =>
    v !== null && typeof v === 'object' ? matches(v, e[k] ?? {}) : e[k] === v,
  )

const run = (argv: readonly string[], cwd: string | undefined, env: NodeJS.ProcessEnv) =>
  new Promise<{ exitCode: number; stdout: string; stderr: string }>((resolve, reject) => {
    execFile(argv[0]!, argv.slice(1), { cwd, env, maxBuffer: 1 << 26 }, (error, stdout, stderr) => {
      if (error && typeof error.code !== 'number') reject(error)
      else resolve({ exitCode: typeof error?.code === 'number' ? error.code : 0, stdout, stderr })
    })
  })

export type Engine = {
  /** The `$` the module's hooks are called with. */
  $: any
  /** Runs a slash command, as the person typing it. */
  command(command: string, args?: string): Promise<{ text: string }>
  /** Moves the clock on, running the module's timers that come due. */
  advance(ms: number): Promise<void>
  /** The pane as drawn after the latest change, one line per row. */
  pane(): string
  /** Presses the pane's button with this label. */
  press(label: string): Promise<void>
}

/**
 * Loads the plugin at `pluginRoot`, whose sessions run in `cwd` with `path` first on `PATH`.
 */
export async function createEngine(opts: {
  pluginRoot: string
  cwd: string
  path: string
}): Promise<Engine> {
  const hooks: Hook[] = []
  const on = (event: string, a: any, b?: Handler) => {
    hooks.push(b ? { event, filter: a, handler: b } : { event, filter: {}, handler: a })
  }
  const manifest = JSON.parse(await readFile(join(opts.pluginRoot, 'hooks/hooks.json'), 'utf8'))
  for (const module of manifest.modules as string[]) {
    const loaded = await import(/* @vite-ignore */ pathToFileURL(join(opts.pluginRoot, 'hooks', module)).href)
    loaded.register(on)
  }

  let now = 0
  const timers: { every: number; next: number; fn: () => void; live: boolean }[] = []
  const panes: { id: string; isPlaced: boolean }[] = []
  let tree: unknown = undefined
  let drawing = Promise.resolve()
  const env = { ...process.env, PATH: `${opts.path}:${process.env.PATH}` }

  const elements = { Box: 'Box', Text: 'Text', Button: 'Button', Code: 'Code' }
  const $: any = {
    __store: new Map(),
    __changed: () => {
      // A re-render on every change, one after another.
      drawing = drawing.then(draw, draw)
    },
    plugin: { root: opts.pluginRoot },
    clock: {
      now: async () => now,
      every(ms: number, fn: () => void) {
        const timer = { every: ms, next: now + ms, fn, live: true }
        timers.push(timer)
        return { cancel: () => void (timer.live = false) }
      },
    },
    session: { cwd: async () => opts.cwd },
    agent: { list: async () => [] },
    process: { run: async (argv: readonly string[], init?: { cwd?: string }) => run(argv, init?.cwd, env) },
    fs: {
      stat: async (path: string) => ({ kind: (await stat(path)).isDirectory() ? 'dir' : 'file' }),
      list: async (path: string) =>
        (await readdir(path, { withFileTypes: true })).map(d => ({
          name: d.name,
          kind: d.isDirectory() ? 'dir' : 'file',
        })),
    },
    ui: {
      panes: async () => panes,
      open: async ({ id }: { id: string }) => {
        panes.push({ id, isPlaced: true })
        $.__changed()
      },
      resolve: (_e: unknown) => elements,
    },
  }

  const call = async (event: string, e: Record<string, any>) => {
    const hook = hooks.find(h => h.event === event && matches(h.filter, e))
    if (!hook) throw new Error(`no hook for ${event} ${JSON.stringify(e)}`)
    return hook.handler($, e, async (x: unknown) => x)
  }

  async function draw() {
    if (panes.length === 0) return
    tree = await call('ui.render', {
      component: 'Pane',
      requestId: panes[0]!.id,
      surface: 'terminal',
      props: { bodyColumns: 100 },
    })
  }

  const text = (n: unknown): string[] => {
    if (n === null || n === undefined || n === false || n === true) return []
    if (typeof n === 'string' || typeof n === 'number') return [String(n)]
    if (Array.isArray(n)) return n.flatMap(text)
    const { type, props, children } = n as Node
    const inner = children.flatMap(text)
    if (type === 'Text') return [inner.join('')]
    if (type === 'Button') return props.label !== undefined ? [`[${props.label}]`] : inner
    if (type === 'Box' && props.flexDirection === 'row') return [inner.join(' ')]
    return inner
  }

  const buttons = (n: unknown): Node[] => {
    if (Array.isArray(n)) return n.flatMap(buttons)
    if (n === null || typeof n !== 'object') return []
    const node = n as Node
    return [...(node.type === 'Button' ? [node] : []), ...node.children.flatMap(buttons)]
  }

  const settle = async () => {
    // Draws started while waiting are waited for too.
    let seen
    do {
      seen = drawing
      await seen
    } while (seen !== drawing)
  }

  return {
    $,
    async command(command, args = '') {
      const answer = await call('command.run', { command, args })
      await settle()
      return answer
    },
    async advance(ms) {
      for (let step = 0; step < ms; step += 1000) {
        now += 1000
        for (const t of timers) {
          while (t.live && now >= t.next) {
            t.next += t.every
            void Promise.resolve(t.fn()).catch(e => { throw e })
          }
        }
        // Let the callbacks the timers started begin before the clock moves again.
        await new Promise(r => setTimeout(r, 0))
      }
    },
    pane: () => text(tree).join('\n'),
    async press(label) {
      const b = buttons(tree).find(b => b.props.label === label)
      if (!b) throw new Error(`no button ${label} in:\n${text(tree).join('\n')}`)
      await b.props.onPress()
      await settle()
    },
  }
}
