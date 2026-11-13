// The build rows' rules: the runs this session launched, kept from the hooks events, their
// estimated cost, and the lines they add under the Flight chart. Pure functions over the run
// list; `hooks/register.tsx` keeps the list in an atom and feeds it the events.

import { AMBER, INDIGO, ROSE, type Span } from './panel.ts'

import type { AgentRun, RunStatus, Task, TaskStatus } from '../types'

export type { AgentRun, RunStatus, Task, TaskStatus } from '../types'

/** How many runs the list keeps. */
export const MAX_RUNS = 200

/**
 * The queue entry a prompt runs: one of its first five lines is `loop run <name>`, the first
 * such line deciding. The loop's launcher puts it on its first line; a launcher quoted further
 * down names no run.
 */
export function runEntry(prompt: string): string | undefined {
  for (const line of prompt.split('\n', 5)) {
    const name = /^loop run (\S+)$/.exec(line.trim())?.[1]
    if (name !== undefined) return name
  }
  return undefined
}

const normalised = (s: string) => s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, ' ').trim()

const updated = (list: readonly AgentRun[], id: string, fn: (a: AgentRun) => AgentRun): AgentRun[] =>
  list.map(a => (a.id === id ? fn(a) : a))

/** A listed run's agent starts a loop again: running, not ended or dismissed, its stats kept. */
const resumed = (list: readonly AgentRun[], id: string): AgentRun[] =>
  updated(list, id, ({ endedAt: _, dismissed: __, ...a }) => ({ ...a, status: 'running' }))

/**
 * The list with a run started: a description a listed run already has is its next round. A
 * spawn that resumes a listed agent keeps its run and its stats, running again.
 */
export function startRun(
  list: readonly AgentRun[],
  r: { id: string; parentId?: string; type: string; description: string; model: string; prompt: string; at: number },
): AgentRun[] {
  if (list.some(a => a.id === r.id)) return resumed(list, r.id)
  const key = normalised(r.description)
  const earlier = key === '' ? 0 : list.filter(a => normalised(a.description) === key).length
  const entry = runEntry(r.prompt)
  const run: AgentRun = {
    id: r.id,
    ...(r.parentId !== undefined && { parentId: r.parentId }),
    type: r.type,
    description: r.description,
    model: r.model,
    ...(entry !== undefined && { entry }),
    status: 'running',
    startedAt: r.at,
    round: earlier + 1,
    contextTokens: 0,
    tokens: 0,
    costUsd: 0,
    steps: 0,
    tasks: [],
  }
  return [...list, run].slice(-MAX_RUNS)
}

/** `/clear` and resume begin a new conversation: the list empties. Quitting keeps it. */
export const resetsOn = (reason: string): boolean => reason === 'clear' || reason === 'resume'

/** After a reload: a run still running or waiting whose agent the engine no longer lists won't finish. */
export const reconcile = (list: readonly AgentRun[], listed: readonly { id: string }[]): AgentRun[] =>
  list.filter(a => (a.status !== 'running' && a.status !== 'waiting') || listed.some(l => l.id === a.id))

/**
 * The engine's status may lag the hooks, so the runs are checked against `$.agent.list()` at
 * each 30-second redraw: a `done` run listed `waiting` is waiting again, and a `waiting` run
 * listed any other way (`running` apart), or not at all, is done, ended at `at`.
 */
export const reconcileWaiting = (
  list: readonly AgentRun[],
  listed: readonly { id: string; status: string }[],
  at: number,
): AgentRun[] =>
  list.map(a => {
    const status = listed.find(l => l.id === a.id)?.status
    if (a.status === 'done' && status === 'waiting') {
      const { endedAt: _, ...rest } = a
      return { ...rest, status: 'waiting' }
    }
    if (a.status === 'waiting' && status !== 'waiting' && status !== 'running') return { ...a, status: 'done', endedAt: at }
    return a
  })

/** One model request's tokens, as the engine reports them. */
export type Usage = {
  input_tokens: number
  output_tokens: number
  cache_read_input_tokens: number
  cache_creation_input_tokens: number
}

const tokensOf = (u: Usage): number =>
  (u.input_tokens || 0) + (u.output_tokens || 0) + (u.cache_read_input_tokens || 0) + (u.cache_creation_input_tokens || 0)

/** A request of a waiting or done run means its loop went on: it runs again. */
const resumedIfIdle = (list: readonly AgentRun[], id: string): AgentRun[] =>
  list.some(a => a.id === id && (a.status === 'waiting' || a.status === 'done')) ? resumed(list, id) : [...list]

/** One model request of a run: its context is that request's; tokens and cost accumulate. */
export const applyStep = (
  list: readonly AgentRun[],
  id: string,
  model: string,
  effort: string | undefined,
  u: Usage,
): AgentRun[] =>
  updated(resumedIfIdle(list, id), id, a => ({
    ...a,
    model,
    effort: effort ?? a.effort,
    contextTokens: tokensOf(u),
    tokens: a.tokens + tokensOf(u),
    costUsd: a.costUsd + costOf(model, u),
    steps: a.steps + 1,
  }))

/**
 * The run's turn ended: `failed` unless on an answer, then `waiting` (no end, so its time
 * grows) while its agent is held on background work, else `done`; a run that saw no step takes
 * the turn's usage.
 */
export const applyComplete = (
  list: readonly AgentRun[],
  id: string,
  isAnswer: boolean,
  at: number,
  u?: Usage & { model?: string },
  waiting = false,
): AgentRun[] =>
  updated(list, id, ({ endedAt: _, ...a }) => {
    const model = u?.model || a.model
    const holds = isAnswer && waiting
    return {
      ...a,
      status: holds ? 'waiting' : isAnswer ? 'done' : 'failed',
      ...(!holds && { endedAt: at }),
      ...(a.steps === 0 && u && { model, contextTokens: tokensOf(u), tokens: tokensOf(u), costUsd: costOf(model, u) }),
    }
  })

type TaskInput = {
  todos?: readonly { content?: string; status?: string }[]
  taskId?: string
  subject?: string
  status?: string
}

const taskStatus = (s: string | undefined): TaskStatus | undefined =>
  s === 'pending' || s === 'in_progress' || s === 'completed' ? s : undefined

/**
 * A tool call of a run feeds its task list: `TodoWrite` replaces it, `TaskCreate` appends a
 * pending task (`createdId` the id the call answered), `TaskUpdate` changes or deletes one.
 */
export function applyTool(
  list: readonly AgentRun[],
  id: string,
  tool: string,
  input: TaskInput,
  createdId?: string,
): AgentRun[] {
  const tasks = (fn: (t: Task[]) => Task[]) => updated(list, id, a => ({ ...a, tasks: fn(a.tasks) }))
  if (tool === 'TodoWrite') {
    return tasks(() =>
      (input.todos ?? []).map((t, i) => ({ id: String(i + 1), content: t.content ?? '', status: taskStatus(t.status) ?? 'pending' })),
    )
  }
  if (tool === 'TaskCreate') {
    return tasks(t => {
      const taskId = createdId ?? String(t.length + 1)
      return [...t.filter(x => x.id !== taskId), { id: taskId, content: input.subject ?? '', status: 'pending' }]
    })
  }
  if (tool === 'TaskUpdate' && input.taskId !== undefined) {
    const { taskId, subject } = input
    if (input.status === 'deleted') return tasks(t => t.filter(x => x.id !== taskId))
    const status = taskStatus(input.status)
    return tasks(t => t.map(x => (x.id === taskId ? { ...x, content: subject ?? x.content, status: status ?? x.status } : x)))
  }
  return [...list]
}

// The hooks events as the list sees them: `register.tsx` passes each event (and what `next`
// answered) through unchanged.

/** `agent.spawn`, after `next`: a started subagent joins the list. */
export const onSpawn = (
  list: readonly AgentRun[],
  e: { prompt: string; description: string; subagentType: string; parentAgentId?: string },
  started: { agentId?: string; model?: string; deny?: unknown },
  at: number,
): AgentRun[] =>
  started.deny !== undefined || started.agentId === undefined
    ? [...list]
    : startRun(list, {
        id: started.agentId,
        ...(e.parentAgentId !== undefined && { parentId: e.parentAgentId }),
        type: e.subagentType,
        description: e.description,
        model: started.model ?? '',
        prompt: e.prompt,
        at,
      })

/** `turn.step` with an `agentId`, after `next`: one model request of that run. */
export const onStep = (
  list: readonly AgentRun[],
  e: { agentId?: string; model: string; effort?: unknown },
  usage: (Usage & { model?: string }) | null | undefined,
): AgentRun[] =>
  e.agentId === undefined || usage == null
    ? [...list]
    : applyStep(list, e.agentId, usage.model || e.model, e.effort === undefined ? undefined : String(e.effort), usage)

/**
 * `turn.complete` with an `agentId`: the run ends, done on an answer unless the engine lists
 * its agent (`listed`, its `AgentStatus`) `waiting` on background work it owns.
 */
export const onComplete = (
  list: readonly AgentRun[],
  e: { agentId?: string; reason: string; usage?: Usage & { model?: string } },
  at: number,
  listed?: string,
): AgentRun[] =>
  e.agentId === undefined ? [...list] : applyComplete(list, e.agentId, e.reason === 'answer', at, e.usage, listed === 'waiting')

/** `turn.start` with an `agentId` a listed run has: its agent's loop starts a turn again. */
export const onTurnStart = (list: readonly AgentRun[], e: { agentId?: string }): AgentRun[] =>
  e.agentId === undefined || !list.some(a => a.id === e.agentId) ? [...list] : resumed(list, e.agentId)

/** `tool.call` with an `agentId`, after `next`: a task tool's successful call. */
export const onToolCall = (
  list: readonly AgentRun[],
  e: TaskInput & { agentId?: string; tool: string },
  ran: { deny?: unknown; isError?: boolean; result?: unknown },
): AgentRun[] =>
  e.agentId === undefined || ran.deny !== undefined || ran.isError || (ran.result as { success?: unknown } | undefined)?.success === false
    ? [...list]
    : applyTool(list, e.agentId, e.tool, e, (ran.result as { task?: { id?: string } } | undefined)?.task?.id)

/** `session.end`: `/clear` and resume empty the list. */
export const onSessionEnd = (list: readonly AgentRun[], reason: string): AgentRun[] => (resetsOn(reason) ? [] : [...list])

/** A failed run's `Dismiss` pressed: it is drawn no more, and stays in the list for rounds and the totals. */
export const dismiss = (list: readonly AgentRun[], id: string): AgentRun[] =>
  updated(list, id, a => (a.status === 'failed' ? { ...a, dismissed: true } : a))

/** Whether the rows tick: a run is still running or waiting. */
export const ticking = (list: readonly AgentRun[]): boolean => list.some(a => a.status === 'running' || a.status === 'waiting')

/** A task list's progress: `k` completed of `n`, and the first in-progress task, else the first pending. */
export function progress(tasks: readonly Task[]): { k: number; n: number; current?: string } {
  const current = (tasks.find(t => t.status === 'in_progress') ?? tasks.find(t => t.status === 'pending'))?.content
  return { k: tasks.filter(t => t.status === 'completed').length, n: tasks.length, ...(current !== undefined && { current }) }
}

type Rates = readonly [input: number, output: number, cacheRead: number, cacheWrite: number]

/** When `PRICES` was last checked against the list prices, as `effort`'s `prices.rs` was. */
export const PRICES_CHECKED = '2026-10-30'

/**
 * USD per million tokens by model ID, most specific first: an estimate, never a bill. The rows
 * of `crates/clm-effort/src/prices.rs`, in its order, so the pane and `clm capabilities` agree.
 */
export const PRICES: readonly (readonly [RegExp, Rates])[] = [
  [/fable-5-1|mythos-5-1/, [10, 50, 0.25, 12.5]],
  [/fable|mythos/, [10, 50, 1, 12.5]],
  [/opus-5-5/, [4, 20, 0.2, 5]],
  [/opus-5/, [5, 25, 0.5, 6.25]],
  [/opus-4-[5-8]/, [5, 25, 0.5, 6.25]],
  [/opus/, [15, 75, 1.5, 18.75]],
  [/sonnet-5/, [2, 10, 0.2, 2.5]],
  [/haiku-5/, [0.1, 0.5, 0.01, 0.125]],
  [/haiku/, [1, 5, 0.1, 1.25]],
]
const FALLBACK: Rates = [3, 15, 0.3, 3.75]

/** The estimated cost in USD of `u` on `model`. */
export function costOf(model: string, u: Usage): number {
  const [i, o, r, w] = PRICES.find(([re]) => re.test(model.toLowerCase()))?.[1] ?? FALLBACK
  return (
    ((u.input_tokens || 0) * i +
      (u.output_tokens || 0) * o +
      (u.cache_read_input_tokens || 0) * r +
      (u.cache_creation_input_tokens || 0) * w) /
    1e6
  )
}

/** The context window that gives a run's fill. */
export const windowOf = (model: string): number => (/haiku/i.test(model) ? 200_000 : 1_000_000)

const ctxPct = (a: AgentRun): number => Math.min(100, Math.round((a.contextTokens / windowOf(a.model)) * 100))

/** A model ID as people say it: `claude-opus-5-5` is `Opus 5.5`. */
export function modelName(id: string): string {
  const m = /(fable|mythos|opus|sonnet|haiku)-(\d+)(?:-(\d{1,2})(?!\d))?/i.exec(id)
  if (!m) return id.replace(/^claude-/, '').replace(/\[.*\]$/, '') || '—'
  const [, family = '', major = '', minor] = m
  return `${family.charAt(0).toUpperCase()}${family.slice(1).toLowerCase()} ${major}${minor ? `.${minor}` : ''}`
}

// Rounded first, so a count just under a unit reads as the next one: `999.6` is `1k`, `999 600` is `1.0M`.
export const fmtTokens = (n: number): string =>
  n >= 999_500 ? `${(n / 1e6).toFixed(1)}M` : n >= 999.5 ? `${Math.round(n / 1e3)}k` : `${Math.round(n)}`

/** To the cent under $10, to the tenth above. */
export const fmtCost = (usd: number): string => {
  const cents = Math.round(usd * 100) / 100
  return `$${cents < 10 ? cents.toFixed(2) : usd.toFixed(1)}`
}

export const fmtTime = (ms: number): string => {
  const s = Math.max(0, Math.round(ms / 1000))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const ss = String(s % 60).padStart(2, '0')
  return h ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`
}

/** A bar of `width` cells filled to `ratio`. */
export const bar = (ratio: number, width = 10): string => {
  const filled = Math.round(width * Math.max(0, Math.min(1, ratio)))
  return '█'.repeat(filled) + '░'.repeat(width - filled)
}

// A run that finishes `done` is never drawn, so it has no glyph.
const GLYPHS: Record<Exclude<RunStatus, 'done'>, Span> = {
  running: { text: '●', color: AMBER },
  waiting: { text: '●', color: AMBER },
  failed: { text: '✗', color: ROSE },
}

/** Only running, waiting and failed runs not dismissed are drawn; a run that finishes `done` leaves the rows. */
const isDrawn = (a: AgentRun): boolean => a.status === 'running' || a.status === 'waiting' || (a.status === 'failed' && a.dismissed !== true)

const elapsed = (a: AgentRun, at: number): number => (a.endedAt ?? Math.max(at, a.startedAt)) - a.startedAt

/**
 * The drawn runs as a tree: each under its nearest drawn ancestor, so a drawn run whose parent
 * isn't drawn takes the parent's place. One with no drawn ancestor is at the top, with the entry
 * of its top-level ancestor, the one whose parent isn't listed.
 */
type Forest = { tops: { run: AgentRun; entry?: string }[]; children: Map<string, AgentRun[]> }

function forest(list: readonly AgentRun[]): Forest {
  const byId = new Map(list.map(a => [a.id, a]))
  const tops: Forest['tops'] = []
  const children = new Map<string, AgentRun[]>()
  for (const a of list) {
    if (!isDrawn(a)) continue
    let top = a
    let under: AgentRun | undefined
    // `seen` guards against a cycle of parents, which the engine never makes.
    const seen = new Set([a.id])
    for (let p = byId.get(a.parentId ?? ''); p && !seen.has(p.id); p = byId.get(p.parentId ?? '')) {
      seen.add(p.id)
      if (isDrawn(p)) {
        under = p
        break
      }
      top = p
    }
    if (under) children.set(under.id, [...(children.get(under.id) ?? []), a])
    else tops.push({ run: a, ...(top.entry !== undefined && { entry: top.entry }) })
  }
  return { tops, children }
}

const newestFirst = (runs: readonly AgentRun[]): AgentRun[] => [...runs].sort((x, y) => y.startedAt - x.startedAt)

/** A run's three lines, `indent` cells in and two more per level of `depth`, up to four. */
function runLines(a: AgentRun, depth: number, at: number, indent: number): Span[][] {
  const pad = ' '.repeat(indent + 2 * Math.min(depth, 4))
  const { k, n, current } = progress(a.tasks)
  const model = [modelName(a.model), a.effort].filter(Boolean).join(' · ')
  const meta = [a.type, model, a.round > 1 ? `round ${a.round}` : ''].filter(Boolean).join(' · ')
  const fill = n > 0 ? { text: bar(k / n), color: AMBER } : { text: bar(ctxPct(a) / 100), dim: true }
  const steps = n > 0 ? `${k}/${n}${current ? ` ${current}` : ''} · ` : ''
  const stats = `${steps}ctx ${ctxPct(a)}% · ${fmtTokens(a.tokens)} tok ≈${fmtCost(a.costUsd)} ${fmtTime(elapsed(a, at))}`
  return [
    [
      { text: pad + (depth > 0 ? '└ ' : '') },
      a.status === 'done' ? GLYPHS.running : GLYPHS[a.status],
      { text: ` ${a.description || a.type}`, bold: true },
      ...(a.status === 'waiting' ? [{ text: ' · waiting', dim: true }] : []),
      ...(a.status === 'failed' ? [{ text: 'Dismiss', button: `dismiss:${a.id}` }] : []),
    ],
    [{ text: `${pad}  ` }, { text: meta, dim: true }],
    [{ text: `${pad}  ` }, fill, { text: ` ${stats}`, dim: true }],
  ]
}

/** Each root's lines, then its drawn subtree depth-first, children oldest first. */
function treeLines(children: Forest['children'], roots: readonly AgentRun[], at: number, indent: number): Span[][] {
  const walk = (a: AgentRun, depth: number): Span[][] => [
    ...runLines(a, depth, at, indent),
    ...[...(children.get(a.id) ?? [])].sort((x, y) => x.startedAt - y.startedAt).flatMap(c => walk(c, depth + 1)),
  ]
  return roots.flatMap(r => walk(r, 0))
}

/** `≈$<cost> · <tokens> tokens · <time> · <model> ×<n>`, per model most used first. */
function totalsLine(list: readonly AgentRun[], at: number): Span[] {
  const start = Math.min(...list.map(a => a.startedAt))
  const end = Math.max(...list.map(a => a.endedAt ?? Math.max(at, a.startedAt)))
  const counts = new Map<string, number>()
  for (const a of list) counts.set(modelName(a.model), (counts.get(modelName(a.model)) ?? 0) + 1)
  const models = [...counts].sort((x, y) => y[1] - x[1]).map(([m, c]) => `${m} ×${c}`)
  const cost = list.reduce((s, a) => s + a.costUsd, 0)
  const tokens = list.reduce((s, a) => s + a.tokens, 0)
  return [{ text: [`≈${fmtCost(cost)}`, `${fmtTokens(tokens)} tokens`, fmtTime(end - start), ...models].join(' · '), dim: true }]
}

/**
 * The drawn rows under no node line, under an `agents` heading drawn only when a row follows,
 * then the totals over every run of the list, drawn or not.
 */
function tailLines(list: readonly AgentRun[], trees: Forest, rest: readonly AgentRun[], at: number): Span[][] {
  if (list.length === 0) return []
  const agents = rest.length === 0 ? [] : [[{ text: 'agents', color: INDIGO }], ...treeLines(trees.children, newestFirst(rest), at, 2)]
  return [...agents, totalsLine(list, at)]
}

export const lineText = (line: readonly Span[]) => line.map(s => s.text).join('')

/**
 * The indexes of the flight chart's component lines (`view/flight-component-lines`): those between
 * the `components` heading, or the old `nodes` heading a `clm` built before the rename still
 * prints (`component-words`), and the `changes` heading.
 */
export function componentRange(texts: readonly string[]): [number, number] {
  const current = texts.indexOf('components')
  const from = current >= 0 ? current : texts.indexOf('nodes')
  const to = from < 0 ? -1 : texts.findIndex((t, i) => i > from && t === 'changes')
  return [from, to]
}

/**
 * The queue chart's running entries (`view/queue-view`) by line index: the lines under the
 * `running` heading that start with two spaces and a name, up to the next heading.
 */
function runningEntries(texts: readonly string[]): Map<number, string> {
  const out = new Map<number, string>()
  const from = texts.indexOf('running')
  if (from < 0) return out
  for (let i = from + 1; i < texts.length && texts[i]!.startsWith(' '); i++) {
    const name = /^ {2}([^ ]+)/.exec(texts[i]!)?.[1]
    if (name !== undefined) out.set(i, name)
  }
  return out
}

/**
 * The queue chart's lines with each running entry's line followed by the rows of its runs,
 * newest first, then the runs under no running entry and the totals. An entry's line starts
 * with two spaces and its name, or a prefix of it followed by `…`.
 */
export function queueLines(chart: readonly Span[][], list: readonly AgentRun[], at: number): Span[][] {
  const entries = runningEntries(chart.map(lineText))
  const exact = new Set([...entries.values()].filter(id => !id.endsWith('…')))
  const trees = forest(list)
  const runs = [...trees.tops].sort((x, y) => y.run.startedAt - x.run.startedAt)
  const placed = new Set<string>()
  const out: Span[][] = []
  chart.forEach((line, i) => {
    out.push(line)
    const id = entries.get(i)
    if (id === undefined) return
    const prefix = id.endsWith('…') ? id.slice(0, -1) : undefined
    const mine = runs
      .filter(
        ({ run, entry }) =>
          entry !== undefined &&
          !placed.has(run.id) &&
          (prefix === undefined ? entry === id : entry.startsWith(prefix) && !exact.has(entry)),
      )
      .map(t => t.run)
    for (const b of mine) placed.add(b.id)
    out.push(...treeLines(trees.children, mine, at, 4))
  })
  const rest = newestFirst(trees.tops.map(t => t.run).filter(a => !placed.has(a.id)))
  return [...out, ...tailLines(list, trees, rest, at)]
}

/**
 * The Queue tab's text lines: an ANSI `chart` with the run rows under its running entries, or,
 * when the text chart is drawn instead (`undefined`), which the pane can't read entries from,
 * the rows that follow it.
 */
export const queueRows = (chart: readonly Span[][] | undefined, list: readonly AgentRun[], at: number): Span[][] =>
  chart === undefined ? followingLines(list, at) : queueLines(chart, list, at)

/**
 * The rows that follow a text chart: every run under a line holding its entry's name, entries in
 * the order of their first run, then the runs that belong to none and the totals.
 */
export function followingLines(list: readonly AgentRun[], at: number): Span[][] {
  const trees = forest(list)
  const { tops } = trees
  const order = [...tops].sort((x, y) => x.run.startedAt - y.run.startedAt).flatMap(t => (t.entry === undefined ? [] : [t.entry]))
  const of = (entry: string | undefined) => newestFirst(tops.filter(t => t.entry === entry).map(t => t.run))
  const out = [...new Set(order)].flatMap(entry => [[{ text: entry, bold: true }], ...treeLines(trees.children, of(entry), at, 2)])
  return [...out, ...tailLines(list, trees, of(undefined), at)]
}
