// The pane's pressable charts on the terminal surface: which ANSI line is which node or root,
// the node inspector drawn above the Spec and Flight charts, and the Blockers tab's folded
// chains. Every fact comes from `clm view <tab> --format json` and every line from `--format
// ansi`; these rules only match the two by position. `hooks/register.tsx` turns what they
// return into elements and presses into atom updates.

import { type AgentRun, componentRange, flightLines, lineText } from './agents.ts'
import { AMBER, GREEN, INDIGO, ROSE, type Run, type Span, type Surface, type ViewTab, viewOf } from './panel.ts'

/** A chart line as drawn; `key` (`node:<id>` or `root:<name>`) makes it a pressable `Button`. */
export type PaneLine = { spans: Span[]; key?: string }

type SpecNode = {
  node: string
  title: string
  status: string
  owners: string[]
  depends_on: string[]
  current: number
  behind: number
  unbuilt: number
  obsolete: number
  total: number
  oversized: boolean
  /** `view/node-detail`: its Contract's text, its live changes and its backlog items. */
  contract?: string
  changes?: { change: string; status: string; current: number; total: number; complete: boolean }[]
  backlog?: { slug: string; kind: string; title: string }[]
}
type Blocker = { item?: string; requirement?: string; requirements: string[] }
type FlightNode = {
  node: string
  priority: string
  state: string
  claim: { pushed_hours_ago: number; lapses_in_hours: number } | null
  blocked_by: Blocker[]
  pending: { id: string; state: string; markers: string[] }[]
}
type Parsed<T> = { items: T[]; checkout: boolean } | { error: string }

// `colour-meanings`, as `ansiLines` names `chart.rs`'s ANSI colours.
const SKY = 'cyan'
const STATE_COLOURS: Record<string, string> = {
  ready: SKY,
  claimed: AMBER,
  blocked: ROSE,
  unbuilt: SKY,
  behind: AMBER,
  obsolete: ROSE,
}

/** The JSON the inspector also needs on `tab`, beside the tab's own. */
export const otherTab = (tab: ViewTab): ViewTab | undefined =>
  tab === 'spec' ? 'flight' : tab === 'flight' ? 'spec' : undefined

/**
 * The `clm view --format json` runs beside the chart: only on the terminal surface,
 * where lines can be pressed; the other tab's only while a node is selected. The Queue tab's
 * lines aren't pressable, so none runs for it (`queue-tab`).
 */
export function modelRuns(
  tab: ViewTab,
  surface: Surface,
  selected: string | null,
  at = false,
): { own?: string[]; other?: string[] } {
  if (surface !== 'terminal' || tab === 'queue') return {}
  const json = (t: ViewTab) => [...viewOf(t, at), '--format', 'json']
  const other = selected === null ? undefined : otherTab(tab)
  return { own: json(tab), ...(other && { other: json(other) }) }
}

/** A `--format json` run's list (`nodes` or `roots`), or why the pane can't read it. */
function parse<T>(tab: ViewTab, run: Run | undefined, list: 'nodes' | 'roots'): Parsed<T> {
  if (run === undefined) return { error: `clm view ${tab} --format json didn't run` }
  if (run.exitCode !== 0) return { error: run.stderr }
  try {
    const model = JSON.parse(run.stdout) as Record<string, unknown>
    const items = model[list]
    // With `--at`, the header line is the commit's (`at`) in place of the checkout's.
    if (Array.isArray(items)) return { items: items as T[], checkout: model.checkout != null || model.at != null }
  } catch {
    // Read as no model, below.
  }
  return { error: `clm view ${tab} --format json printed no model the pane can read` }
}

/**
 * The indexes of the chart's pressable lines (`view/spec-component-lines`, `view/flight-component-lines`,
 * `view/blockers-root-lines`), in order.
 */
function lineIndexes(tab: ViewTab, chart: readonly Span[][], checkout: boolean): number[] {
  const texts = chart.map(lineText)
  const all = texts.map((_, i) => i)
  if (tab === 'spec') return all.slice(checkout ? 1 : 0)
  if (tab === 'flight') {
    const [nodes, changes] = componentRange(texts)
    return all.filter(i => i > nodes && i < changes && texts[i]!.startsWith('  ') && texts[i] !== '  none')
  }
  const heading = texts.indexOf('blockers')
  return all.filter(i => heading >= 0 && i > heading && /^[^ ├└│]/.test(texts[i]!) && texts[i] !== 'nothing is blocked')
}

/** Pressing a node's line: selects it, or clears the selection when it is the selected one. */
export const pressNode = (selected: string | null, id: string): string | null => (selected === id ? null : id)

/** Pressing a root's line: unfolds it, or folds it when it is unfolded. */
export const toggleRoot = (unfolded: readonly string[], name: string): string[] =>
  unfolded.includes(name) ? unfolded.filter(n => n !== name) : [...unfolded, name]

/** What a pressed line's or button's key names; `dismiss:<id>` is a failed build row's `Dismiss`. */
export const keyTarget = (
  key: string,
): { node: string } | { root: string } | { dismiss: string } | undefined =>
  key.startsWith('node:') ? { node: key.slice(5) }
  : key.startsWith('root:') ? { root: key.slice(5) }
  : key.startsWith('dismiss:') ? { dismiss: key.slice(8) }
  : undefined

export type ChartInput = {
  tab: ViewTab
  /** `clm view <tab> --format ansi`, as `ansiLines` reads it. */
  chart: Span[][]
  /** `clm view <tab> --format json`. */
  own: Run
  /** The other tab's JSON (`otherTab`), run only while a node is selected. */
  other?: Run
  selected: string | null
  unfolded: readonly string[]
  /** The pane's columns, which the contract wraps to; 100 when it isn't a positive number. */
  columns?: number
  /** Whether the inspector shows the node's contract (the `contract` atom). */
  contract?: boolean
}

/**
 * What the inspector draws after its lines (`inspector-detail`): the `changes` and `backlog`
 * lines, the `Contract` button's variant, and the contract's lines while it shows.
 */
export type Detail = { lines: Span[][]; button: 'primary' | 'secondary'; contract?: string[] }

export type Chart = { lines: PaneLine[]; inspector?: Span[][]; detail?: Detail }

/**
 * The tab's lines, its node or root lines pressable and its chains folded, and the inspector
 * to draw above them. When the lines can't be matched to the model the chart is drawn as it
 * came: nothing pressable, no inspector, every chain shown.
 */
export function chartLines(input: ChartInput): Chart {
  const { tab, chart, own, selected, unfolded } = input
  const plain = { lines: chart.map(spans => ({ spans })) }
  const model = parse<{ node?: string; root?: string }>(tab, own, tab === 'blockers' ? 'roots' : 'nodes')
  if ('error' in model) return plain
  const at = lineIndexes(tab, chart, model.checkout)
  if (at.length !== model.items.length) return plain
  const names = new Map(at.map((line, n) => [line, String(model.items[n]![tab === 'blockers' ? 'root' : 'node'])]))

  if (tab === 'blockers') {
    const lines: PaneLine[] = []
    let shown = true
    chart.forEach((spans, i) => {
      const name = names.get(i)
      if (name !== undefined) {
        shown = unfolded.includes(name)
        lines.push({ spans, key: `root:${name}` })
      } else if (shown) lines.push({ spans })
    })
    return { lines }
  }

  const lines = chart.map((spans, i): PaneLine => {
    const id = names.get(i)
    if (id === undefined) return { spans }
    return { spans: id === selected ? spans.map(s => ({ ...s, bold: true })) : spans, key: `node:${id}` }
  })
  if (selected === null) return { lines }
  const spec = tab === 'spec' ? (model as Parsed<SpecNode>) : parse<SpecNode>('spec', input.other, 'nodes')
  const flight = tab === 'flight' ? (model as Parsed<FlightNode>) : parse<FlightNode>('flight', input.other, 'nodes')
  // The selected node, or why the spec model can't say; a node it doesn't list draws no inspector.
  const n = 'error' in spec ? spec : spec.items.find(s => s.node === selected)
  if (n === undefined) return { lines }
  const inspector = inspect(selected, n, flight)
  // A `clm` from before `view/node-detail` gives none of the detail: nothing to say, not "none".
  if ('error' in n || !n.changes || !n.backlog || n.contract === undefined) return { lines, inspector }
  return { lines, inspector, detail: detail(n, input.contract === true, input.columns ?? NaN) }
}

/**
 * The chart as `chartLines` draws it when its JSON model ran (`modelRuns` gave `own`), else
 * as it came: on other surfaces nothing is pressable.
 */
export function paneChart(input: Omit<ChartInput, 'own'> & { own?: Run }): Chart {
  const { own } = input
  return own === undefined ? { lines: input.chart.map(spans => ({ spans })) } : chartLines({ ...input, own })
}

/** The inspector's lines; the `Close` button follows the first. */
function inspect(id: string, n: SpecNode | { error: string }, flight: Parsed<FlightNode>): Span[][] {
  const head: Span[] = [{ text: id, color: INDIGO, bold: true }]
  if (!('error' in n)) head.push({ text: `  ${n.title}` })
  const facts = 'error' in n ? [[{ text: n.error }]] : specFacts(n)
  return [head, ...facts, ...('error' in flight ? [[{ text: flight.error }]] : inFlight(flight.items.find(f => f.node === id)))]
}

function specFacts(n: SpecNode): Span[][] {
  const owners = n.owners.length > 0 ? n.owners.join(', ') : 'none'
  const deps = n.depends_on.length > 0 ? n.depends_on.join(', ') : 'nothing'
  const counts = [`requirements ${n.current}/${n.total} current`]
  for (const state of ['behind', 'unbuilt', 'obsolete'] as const) if (n[state] > 0) counts.push(`, ${n[state]} ${state}`)
  return [
    [{ text: [n.status, `owners ${owners}`, `depends on ${deps}`].join(' · ') }],
    [{ text: counts.join('') }, ...(n.oversized ? [{ text: '  ⚠ oversized', color: AMBER }] : [])],
  ]
}

function detail(n: SpecNode, shown: boolean, columns: number): Detail {
  const heading = (name: string, rows: Span[][]): Span[][] =>
    rows.length === 0 ? [[{ text: name, color: INDIGO }, { text: ' none' }]] : [[{ text: name, color: INDIGO }], ...rows]
  const changes = (n.changes ?? []).map(c => [
    { text: `  ${c.change} ${c.status} ${c.current}/${c.total} requirements` },
    ...(c.complete ? [{ text: ' ' }, { text: 'complete', color: GREEN }] : []),
  ])
  const backlog = (n.backlog ?? []).map(i => [
    { text: `  backlog/${i.slug} ` },
    { text: i.kind, dim: true },
    { text: ` ${i.title}` },
  ])
  return {
    lines: [...heading('changes', changes), ...heading('backlog', backlog)],
    button: shown ? 'primary' : 'secondary',
    ...(shown && { contract: wrapText(n.contract ?? '', columns) }),
  }
}

/**
 * `text` as plain lines no wider than `width` (100 when it isn't a positive number), one
 * character to a cell: each line broken at spaces, a word longer than the width cut across
 * lines, blank lines kept.
 */
export function wrapText(text: string, width: number): string[] {
  const w = Number.isFinite(width) && Math.floor(width) > 0 ? Math.floor(width) : 100
  const out: string[] = []
  for (const line of text.split(/\r?\n/)) {
    let current = ''
    for (let word of line.split(/[ \t]+/).filter(x => x !== '')) {
      while (word.length > w) {
        if (current !== '') out.push(current)
        current = ''
        out.push(word.slice(0, w))
        word = word.slice(w)
      }
      if (word === '') continue
      if (current === '') current = word
      else if (current.length + 1 + word.length <= w) current += ` ${word}`
      else {
        out.push(current)
        current = word
      }
    }
    out.push(current)
  }
  return out
}

function inFlight(f: FlightNode | undefined): Span[][] {
  if (f === undefined) return [[{ text: 'converged', color: GREEN }]]
  const rest = [f.priority]
  if (f.claim) rest.push(`pushed ${f.claim.pushed_hours_ago}h ago, lapses in ${f.claim.lapses_in_hours}h`)
  return [
    [{ text: f.state, color: STATE_COLOURS[f.state] }, { text: ` · ${rest.join(' · ')}` }],
    ...f.blocked_by.map(b => [{ text: `blocked by ${b.item ?? b.requirement ?? '?'} (${b.requirements.join(', ')})` }]),
    ...f.pending.map(r => [
      { text: `  ${r.id} ` },
      { text: r.state, color: STATE_COLOURS[r.state] },
      ...(r.state === 'obsolete' && r.markers.length > 0 ? [{ text: ` ${r.markers.join(' ')}` }] : []),
    ]),
  ]
}

/** The Flight tab's lines with the build rows (`build-rows`) under its node lines, keys kept. */
export function withBuildRows(lines: readonly PaneLine[], runs: readonly AgentRun[], at: number): PaneLine[] {
  const keyOf = new Map(lines.map(l => [l.spans, l.key]))
  return flightLines(
    lines.map(l => l.spans),
    runs,
    at,
  ).map(spans => {
    const key = keyOf.get(spans)
    return key === undefined ? { spans } : { spans, key }
  })
}
