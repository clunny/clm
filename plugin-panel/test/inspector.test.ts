import { describe, expect, test } from 'vitest'

import type { AgentRun } from '../src/agents.ts'
import {
  type PaneLine,
  chartLines,
  keyTarget,
  modelRuns,
  paneChart,
  otherTab,
  pressNode,
  toggleRoot,
  wrapText,
  withBuildRows,
} from '../src/inspector.ts'
import { type Run, type Span, ansiLines, cachedRun, viewArgs, viewKey } from '../src/panel.ts'

const ok = (stdout: string): Run => ({ exitCode: 0, stdout, stderr: '' })
const failed = (stderr: string): Run => ({ exitCode: 1, stdout: '', stderr })
const text = (spans: readonly Span[]) => spans.map(s => s.text).join('')
const texts = (lines: readonly PaneLine[]) => lines.map(l => text(l.spans))
const keys = (lines: readonly PaneLine[]) => lines.flatMap(l => (l.key ? [l.key] : []))
const inspectorText = (inspector: readonly Span[][] | undefined) => (inspector ?? []).map(text).join('\n')

const SPEC_CHART = ansiLines(
  'commit abc1234\n' +
    'r                \x1b[32mactive\x1b[0m  ███ 3/3\n' +
    '├─ a             \x1b[32mactive\x1b[0m  ███ 3/4\n' +
    '└─ b             \x1b[32mactive\x1b[0m  ███ 1/1\n',
)
const node = (id: string, extra: object = {}) => ({
  node: id,
  parent: null,
  title: id.toUpperCase(),
  status: 'active',
  owners: [],
  depends_on: [],
  current: 1,
  behind: 0,
  unbuilt: 0,
  obsolete: 0,
  total: 1,
  oversized: false,
  ...extra,
})
const A = node('a', { title: 'Alpha', owners: ['@o'], depends_on: ['b'], current: 3, total: 4, unbuilt: 1 })
const specJson = (nodes: object[], checkout = true) =>
  ok(JSON.stringify({ view: 'spec', nodes, ...(checkout && { checkout: { commit: 'abc1234', behind: 0 } }) }))
const SPEC = specJson([node('r'), A, node('b')])

const FLIGHT_CHART = ansiLines(
  'commit abc1234\n' +
    'flight  1 node pending\n' +
    '\x1b[34mcomponents\x1b[0m\n' +
    '  a  P5  \x1b[36mready\x1b[0m  ░ 1 pending\n' +
    '\x1b[34mchanges\x1b[0m\n' +
    '  none\n',
)
const flightA = {
  node: 'a',
  priority: 'P5',
  state: 'ready',
  claim: null,
  blocked_by: [],
  pending: [{ id: 'a/x', version: '0', state: 'unbuilt', cited: [], markers: [] }],
}
const flightJson = (nodes: object[]) => ok(JSON.stringify({ view: 'flight', nodes, changes: [] }))
const FLIGHT = flightJson([flightA])

const BLOCKERS_CHART = ansiLines(
  'commit abc1234\n' +
    '\x1b[34mblockers\x1b[0m\n' +
    'gate  claimed  holds up 2\n' +
    '├─ c\n' +
    '│  └─ d\n' +
    'host  claimed  holds up 1\n' +
    '└─ e\n',
)
const root = (name: string) => ({ root: name, kind: 'node', state: 'claimed', waits_on_human: false, weight: 1, holds: [] })
const blockersJson = (roots: object[]) => ok(JSON.stringify({ view: 'blockers', roots }))
const BLOCKERS = blockersJson([root('gate'), root('host')])

describe('node-inspector', () => {
  // req:plugin.panel/component-inspector@d5852e70
  test("each node line of the Spec chart is pressable, by position, with the same text and colours", () => {
    const { lines, inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, selected: null, unfolded: [] })
    expect(inspector).toBeUndefined()
    expect(texts(lines)).toEqual(SPEC_CHART.map(text))
    expect(lines.map(l => l.key)).toEqual([undefined, 'node:r', 'node:a', 'node:b'])
    expect(lines[2]!.spans).toEqual(SPEC_CHART[2])
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('select a node: the inspector holds its spec and its work in flight, above the chart', () => {
    const { lines, inspector } = chartLines({
      tab: 'spec',
      chart: SPEC_CHART,
      own: SPEC,
      other: FLIGHT,
      selected: 'a',
      unfolded: [],
    })
    const [head, facts, counts, flight, pending] = (inspector ?? []).map(text)
    expect(head).toBe('a  Alpha')
    expect(inspector![0]![0]).toEqual({ text: 'a', color: 'blue', bold: true })
    expect(facts).toBe('active · owners @o · depends on b')
    expect(counts).toBe('requirements 3/4 current, 1 unbuilt')
    expect(flight).toBe('ready · P5')
    expect(pending).toBe('  a/x unbuilt')
    expect(inspector![4]!.find(s => s.text === 'unbuilt')).toEqual({ text: 'unbuilt', color: 'cyan' })
    // `a`'s line is still in the chart, pressable and bold.
    expect(lines[2]!.key).toBe('node:a')
    expect(text(lines[2]!.spans)).toBe(text(SPEC_CHART[2]!))
    expect(lines[2]!.spans.every(s => s.bold)).toBe(true)
    expect(lines[3]!.spans.some(s => s.bold)).toBe(false)
  })

  test('counts name each non-zero state, and an oversized node says so in amber', () => {
    const big = node('a', { current: 1, behind: 2, unbuilt: 3, obsolete: 4, total: 10, oversized: true, depends_on: [] })
    const { inspector } = chartLines({
      tab: 'spec',
      chart: SPEC_CHART,
      own: specJson([node('r'), big, node('b')]),
      other: flightJson([]),
      selected: 'a',
      unfolded: [],
    })
    expect(text(inspector![1]!)).toBe('active · owners none · depends on nothing')
    expect(text(inspector![2]!)).toBe('requirements 1/10 current, 2 behind, 3 unbuilt, 4 obsolete  ⚠ oversized')
    expect(inspector![2]!.at(-1)).toEqual({ text: '  ⚠ oversized', color: 'yellow' })
  })

  test('the work in flight: the claim, its blockers, and an obsolete requirement\'s markers', () => {
    const flight = flightJson([
      {
        ...flightA,
        state: 'claimed',
        claim: { lapses_at: 0, pushed_hours_ago: 1, lapses_in_hours: 3 },
        blocked_by: [{ item: 'backlog/q', requirements: ['a/x', 'a/y'] }, { requirement: 'b/z', requirements: ['a/w'] }],
        pending: [
          { id: 'a/x', state: 'behind', markers: [] },
          { id: 'a/y', state: 'obsolete', markers: ['src/a.rs:3', 'tests/a.rs:9'] },
        ],
      },
    ])
    const { inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: flight, selected: 'a', unfolded: [] })
    expect(inspectorText(inspector).split('\n').slice(3)).toEqual([
      'claimed · P5 · pushed 1h ago, lapses in 3h',
      'blocked by backlog/q (a/x, a/y)',
      'blocked by b/z (a/w)',
      '  a/x behind',
      '  a/y obsolete src/a.rs:3 tests/a.rs:9',
    ])
    expect(inspector![3]![0]).toEqual({ text: 'claimed', color: 'yellow' })
    expect(inspector![6]!.find(s => s.text === 'behind')?.color).toBe('yellow')
    expect(inspector![7]!.find(s => s.text === 'obsolete')?.color).toBe('red')
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('no waiting field: without `waiting_on`, or with it, nothing says waiting', () => {
    // The flight model as `view` gives it after `needs-alone-order`, and as older builds did.
    for (const extra of [{}, { waiting_on: ['b'] }]) {
      const other = flightJson([{ ...flightA, ...extra }])
      const { inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other, selected: 'a', unfolded: [] })
      const lines = inspectorText(inspector).split('\n')
      expect(lines.slice(3)).toEqual(['ready · P5', '  a/x unbuilt'])
    }
  })

  test('no waiting state: a `waiting` node takes no state colour', () => {
    const other = flightJson([{ ...flightA, state: 'waiting' }])
    const { inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other, selected: 'a', unfolded: [] })
    expect(inspector![3]![0]).toEqual({ text: 'waiting', color: undefined })
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('a cut ID matches by position', () => {
    const chart = ansiLines('commit abc\nr  active\n├─ al…  active\n└─ b  active\n')
    const { lines, inspector } = chartLines({
      tab: 'spec',
      chart,
      own: specJson([node('r'), node('alongname', { title: 'Long' }), node('b')]),
      other: flightJson([]),
      selected: 'alongname',
      unfolded: [],
    })
    expect(lines[2]!.key).toBe('node:alongname')
    expect(keyTarget('node:alongname')).toEqual({ node: 'alongname' })
    expect(text(inspector![0]!)).toBe('alongname  Long')
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('converged: a node the flight model does not list', () => {
    const { inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: FLIGHT, selected: 'b', unfolded: [] })
    expect(inspector!.slice(3)).toEqual([[{ text: 'converged', color: 'green' }]])
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('pressing the selected line again, or Close, deselects', () => {
    expect(pressNode(null, 'a')).toBe('a')
    expect(pressNode('b', 'a')).toBe('a')
    expect(pressNode('a', 'a')).toBeNull()
    const none = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: FLIGHT, selected: null, unfolded: [] })
    expect(none.inspector).toBeUndefined()
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('the selection holds on the Flight tab, whose component lines between components and changes are pressable', () => {
    expect(otherTab('spec')).toBe('flight')
    expect(otherTab('flight')).toBe('spec')
    expect(otherTab('blockers')).toBeUndefined()
    const { lines, inspector } = chartLines({
      tab: 'flight',
      chart: FLIGHT_CHART,
      own: FLIGHT,
      other: SPEC,
      selected: 'a',
      unfolded: [],
    })
    expect(keys(lines)).toEqual(['node:a'])
    expect(text(lines[3]!.spans)).toBe(text(FLIGHT_CHART[3]!))
    expect(text(inspector![0]!)).toBe('a  Alpha')
    expect(text(inspector![3]!)).toBe('ready · P5')
  })

  test("Flight's `  none` is no node line", () => {
    const chart = ansiLines('commit abc\ncomponents\n  none\nchanges\n  none\n')
    const { lines } = chartLines({ tab: 'flight', chart, own: flightJson([]), selected: null, unfolded: [] })
    expect(keys(lines)).toEqual([])
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('a selected node the spec model no longer lists draws no inspector, and draws one when it comes back', () => {
    const gone = chartLines({
      tab: 'spec',
      chart: ansiLines('commit abc\nr  active\n└─ b  active\n'),
      own: specJson([node('r'), node('b')]),
      other: FLIGHT,
      selected: 'a',
      unfolded: [],
    })
    expect(gone.inspector).toBeUndefined()
    expect(keys(gone.lines)).toEqual(['node:r', 'node:b'])
    const back = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: FLIGHT, selected: 'a', unfolded: [] })
    expect(text(back.inspector![0]!)).toBe('a  Alpha')
  })

  // req:plugin.panel/component-inspector@d5852e70
  test('lines and model disagree: the chart as text, nothing pressable, no inspector', () => {
    const { lines, inspector } = chartLines({
      tab: 'spec',
      chart: SPEC_CHART,
      own: specJson([node('r'), A, node('b'), node('c')]),
      other: FLIGHT,
      selected: 'a',
      unfolded: [],
    })
    expect(inspector).toBeUndefined()
    expect(keys(lines)).toEqual([])
    expect(lines.map(l => l.spans)).toEqual(SPEC_CHART)
  })

  test("the tab's own model failing, or not parsing, falls back the same way", () => {
    for (const own of [failed('boom'), ok('not json'), ok('{"view":"spec"}')]) {
      const { lines, inspector } = chartLines({ tab: 'spec', chart: SPEC_CHART, own, other: FLIGHT, selected: 'a', unfolded: [] })
      expect(inspector).toBeUndefined()
      expect(keys(lines)).toEqual([])
    }
  })

  test('a chart with no checkout line skips none', () => {
    const chart = ansiLines('r  active\n├─ a  active\n└─ b  active\n')
    const { lines } = chartLines({ tab: 'spec', chart, own: specJson([node('r'), A, node('b')], false), selected: null, unfolded: [] })
    expect(keys(lines)).toEqual(['node:r', 'node:a', 'node:b'])
  })

  test("the other tab's model failing: the inspector keeps the ID, Close and its own part, and that run's stderr", () => {
    const onSpec = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: failed('boom'), selected: 'a', unfolded: [] })
    expect(inspectorText(onSpec.inspector)).toBe(
      'a  Alpha\nactive · owners @o · depends on b\nrequirements 3/4 current, 1 unbuilt\nboom',
    )
    const onFlight = chartLines({
      tab: 'flight',
      chart: FLIGHT_CHART,
      own: FLIGHT,
      other: failed('spec broke'),
      selected: 'a',
      unfolded: [],
    })
    expect(inspectorText(onFlight.inspector)).toBe('a\nspec broke\nready · P5\n  a/x unbuilt')
    const unparsed = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: ok('nope'), selected: 'a', unfolded: [] })
    expect(inspectorText(unparsed.inspector)).toContain("clm view flight --format json printed no model the pane can read")
  })

  test('the JSON runs beside the ANSI chart only, the other tab\'s only while a node is selected', () => {
    const json = (tab: string) => ['view', tab, '--format', 'json']
    expect(modelRuns('spec', 'terminal', null)).toEqual({ own: json('spec') })
    expect(modelRuns('spec', 'terminal', 'a')).toEqual({ own: json('spec'), other: json('flight') })
    expect(modelRuns('flight', 'terminal', 'a')).toEqual({ own: json('flight'), other: json('spec') })
    expect(modelRuns('blockers', 'terminal', 'a')).toEqual({ own: json('blockers') })
  })

  test('no presses on the desktop: no JSON runs, so no pressable line and no inspector', () => {
    expect(modelRuns('spec', 'desktop', 'a')).toEqual({})
    expect(modelRuns('blockers', 'vscode', null)).toEqual({})
    for (const tab of ['spec', 'blockers'] as const) {
      const chart = tab === 'spec' ? SPEC_CHART : BLOCKERS_CHART
      const model = tab === 'spec' ? SPEC : BLOCKERS
      const own = modelRuns(tab, 'desktop', 'a').own && model
      const { lines, inspector } = paneChart({ tab, chart, own, selected: 'a', unfolded: [] })
      expect(inspector).toBeUndefined()
      expect(keys(lines)).toEqual([])
      expect(lines.map(l => l.spans)).toEqual(chart)
    }
    expect(paneChart({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: FLIGHT, selected: 'a', unfolded: [] }).inspector).toBeDefined()
  })

  // req:plugin.panel/views-panel@4f33137b
  test('a press reruns nothing the pane already holds this generation', async () => {
    const cache = new Map<string, Promise<Run>>()
    const started: string[] = []
    const draw = async (selected: string | null) => {
      const runs = modelRuns('spec', 'terminal', selected)
      for (const args of [viewArgs('spec', 'terminal', 80), runs.own, runs.other]) {
        if (args) await cachedRun(cache, viewKey(7, args), async () => (started.push(args.join(' ')), ok('')))
      }
    }
    await draw('a')
    expect(started).toEqual([
      'view spec --format ansi --width 80',
      'view spec --format json',
      'view flight --format json',
    ])
    await draw('b')
    await draw(null)
    expect(started).toHaveLength(3)
  })

  test('the Flight tab keeps its build rows under the pressable lines', () => {
    const { lines } = chartLines({ tab: 'flight', chart: FLIGHT_CHART, own: FLIGHT, selected: null, unfolded: [] })
    const build: AgentRun = {
      id: 'x',
      type: 'general-purpose',
      description: 'build a',
      model: 'claude-opus-5-5',
      node: 'a',
      status: 'running',
      startedAt: 0,
      round: 1,
      contextTokens: 0,
      tokens: 0,
      costUsd: 0,
      steps: 0,
      tasks: [],
    }
    const drawn = withBuildRows(lines, [build], 1000)
    const at = drawn.findIndex(l => l.key === 'node:a')
    expect(text(drawn[at + 1]!.spans)).toContain('build a')
    expect(drawn[at + 1]!.key).toBeUndefined()
    expect(keys(drawn)).toEqual(['node:a'])
  })
})

describe('inspector-detail', () => {
  const CONTRACT =
    'The node does one thing well and says so plainly, so that a reader who opens the inspector can ' +
    'tell at a glance what it promises to the rest of the system, what it leaves to its children, and ' +
    'what nobody should rely on, without opening its spec in another window at all.'
  const detailed = (contract = CONTRACT) =>
    specJson([
      node('r'),
      {
        ...A,
        contract,
        changes: [{ change: '20260101-c', status: 'applied', current: 1, total: 2, complete: false }],
        backlog: [{ slug: 'q', kind: 'question', title: 'Ask' }],
      },
      node('b', { contract: 'B does b.', changes: [], backlog: [] }),
    ])
  const draw = (selected: string, contract: boolean, extra: object = {}) =>
    chartLines({
      tab: 'spec',
      chart: SPEC_CHART,
      own: detailed(),
      other: FLIGHT,
      selected,
      unfolded: [],
      columns: 80,
      contract,
      ...extra,
    })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('changes and backlog: one line each after the work in flight, the Contract button secondary, no contract text', () => {
    const { inspector, detail } = draw('a', false)
    expect(inspectorText(inspector)).toBe(
      'a  Alpha\nactive · owners @o · depends on b\nrequirements 3/4 current, 1 unbuilt\nready · P5\n  a/x unbuilt',
    )
    expect(detail!.lines.map(text)).toEqual([
      'changes',
      '  20260101-c applied 1/2 requirements',
      'backlog',
      '  backlog/q question Ask',
    ])
    expect(detail!.lines[0]![0]).toEqual({ text: 'changes', color: 'blue' })
    expect(detail!.lines[3]!.find(s => s.text === 'question')).toEqual({ text: 'question', dim: true })
    expect(detail!.button).toBe('secondary')
    expect(detail!.contract).toBeUndefined()
    expect(JSON.stringify(detail)).not.toContain('promises')
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('a complete change says so in green', () => {
    const own = specJson([
      node('r'),
      { ...A, contract: '', changes: [{ change: 'x', status: 'applied', current: 2, total: 2, complete: true }], backlog: [] },
      node('b'),
    ])
    const { detail } = chartLines({ tab: 'spec', chart: SPEC_CHART, own, other: FLIGHT, selected: 'a', unfolded: [] })
    expect(text(detail!.lines[1]!)).toBe('  x applied 2/2 requirements complete')
    expect(detail!.lines[1]!.at(-1)).toEqual({ text: 'complete', color: 'green' })
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('nothing touches it: changes none and backlog none', () => {
    const { detail } = draw('b', false)
    expect(detail!.lines.map(text)).toEqual(['changes none', 'backlog none'])
    expect(detail!.lines[0]).toEqual([{ text: 'changes', color: 'blue' }, { text: ' none' }])
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('contract shown: the button primary and the paragraph on lines no wider than 80, broken at spaces', () => {
    const { detail } = draw('a', true)
    expect(detail!.button).toBe('primary')
    const lines = detail!.contract!
    expect(lines.length).toBeGreaterThan(3)
    for (const l of lines) expect(l.length).toBeLessThanOrEqual(80)
    expect(lines.join(' ')).toBe(CONTRACT)
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('contract hidden again, and the next node draws its button secondary too', () => {
    expect(draw('a', false).detail!.button).toBe('secondary')
    const b = draw('b', false).detail!
    expect(b.button).toBe('secondary')
    expect(b.contract).toBeUndefined()
    // Whether it shows holds across nodes: the atom isn't the node's.
    expect(draw('b', true).detail!.contract).toEqual(['B does b.'])
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('the spec model failing on the Flight tab: its stderr, and no changes, backlog or Contract button', () => {
    const { inspector, detail } = chartLines({
      tab: 'flight',
      chart: FLIGHT_CHART,
      own: FLIGHT,
      other: failed('boom'),
      selected: 'a',
      unfolded: [],
      columns: 80,
      contract: true,
    })
    expect(inspectorText(inspector)).toContain('boom')
    expect(detail).toBeUndefined()
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('a model without the detail, from an older clm, draws the inspector without it', () => {
    const { inspector, detail } = chartLines({ tab: 'spec', chart: SPEC_CHART, own: SPEC, other: FLIGHT, selected: 'a', unfolded: [] })
    expect(inspector).toBeDefined()
    expect(detail).toBeUndefined()
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('no inspector, no detail: nothing selected, or a node the model does not list', () => {
    expect(draw('a', true, { selected: null }).detail).toBeUndefined()
    expect(draw('gone', true).detail).toBeUndefined()
  })

  // req:plugin.panel/views-panel@4f33137b
  test('pressing Contract reruns nothing the pane already holds this generation', async () => {
    const cache = new Map<string, Promise<Run>>()
    const started: string[] = []
    const runs = async () => {
      const models = modelRuns('spec', 'terminal', 'a')
      const [chart, own, other] = await Promise.all(
        [viewArgs('spec', 'terminal', 80), models.own!, models.other!].map(args =>
          cachedRun(cache, viewKey(3, args), async () => (started.push(args.join(' ')), args[3] === 'json' ? (args[1] === 'spec' ? detailed() : FLIGHT) : ok(''))),
        ),
      )
      return { chart: chart!, own: own!, other: other! }
    }
    const before = await runs()
    expect(chartLines({ tab: 'spec', chart: SPEC_CHART, own: before.own, other: before.other, selected: 'a', unfolded: [], contract: false }).detail!.button).toBe('secondary')
    const after = await runs()
    expect(chartLines({ tab: 'spec', chart: SPEC_CHART, own: after.own, other: after.other, selected: 'a', unfolded: [], contract: true }).detail!.button).toBe('primary')
    expect(started).toHaveLength(3)
  })

  // req:plugin.panel/inspector-detail@7b641ee8
  test('wrapText: at spaces, a long word cut across lines, blank lines kept, 100 for an unmeasured pane', () => {
    expect(wrapText('aa bb cc', 5)).toEqual(['aa bb', 'cc'])
    expect(wrapText('abcdefghij x', 4)).toEqual(['abcd', 'efgh', 'ij x'])
    expect(wrapText('one\n\ntwo', 10)).toEqual(['one', '', 'two'])
    expect(wrapText('a  b   c', 3)).toEqual(['a b', 'c'])
    expect(wrapText('x '.repeat(100).trim(), NaN).every(l => l.length <= 100)).toBe(true)
    expect(wrapText('x '.repeat(100).trim(), NaN)[0]).toHaveLength(99)
    expect(wrapText('x '.repeat(10).trim(), 0)).toEqual(['x '.repeat(10).trim()])
    expect(wrapText('ab cd', 0.5)).toEqual(['ab cd'])
    expect(wrapText('ab\r\ncd\tef', 5)).toEqual(['ab', 'cd ef'])
  })
})

describe('blocker-chains', () => {
  // req:plugin.panel/blocker-chains@2f2eac74
  test('folded at first: the heading and the root lines, pressable, and no tree line', () => {
    const { lines, inspector } = chartLines({ tab: 'blockers', chart: BLOCKERS_CHART, own: BLOCKERS, selected: 'a', unfolded: [] })
    expect(inspector).toBeUndefined()
    expect(texts(lines)).toEqual(['commit abc1234', 'blockers', 'gate  claimed  holds up 2', 'host  claimed  holds up 1'])
    expect(keys(lines)).toEqual(['root:gate', 'root:host'])
    expect(lines[2]!.spans).toEqual(BLOCKERS_CHART[2])
    expect(keyTarget('root:backlog/q')).toEqual({ root: 'backlog/q' })
  })

  test('unfold one: its tree follows its line, the other stays folded', () => {
    const unfolded = toggleRoot([], 'gate')
    expect(unfolded).toEqual(['gate'])
    const { lines } = chartLines({ tab: 'blockers', chart: BLOCKERS_CHART, own: BLOCKERS, selected: null, unfolded })
    expect(texts(lines).slice(2)).toEqual(['gate  claimed  holds up 2', '├─ c', '│  └─ d', 'host  claimed  holds up 1'])
  })

  test('fold again', () => {
    expect(toggleRoot(['host', 'gate'], 'gate')).toEqual(['host'])
    const { lines } = chartLines({ tab: 'blockers', chart: BLOCKERS_CHART, own: BLOCKERS, selected: null, unfolded: [] })
    expect(lines).toHaveLength(4)
  })

  test('a failed or mismatched run draws every line, unfolded and not pressable; a later good one keeps the state', () => {
    for (const own of [failed('boom'), ok('{'), blockersJson([root('gate')])]) {
      const { lines } = chartLines({ tab: 'blockers', chart: BLOCKERS_CHART, own, selected: null, unfolded: ['gate'] })
      expect(lines.map(l => l.spans)).toEqual(BLOCKERS_CHART)
      expect(keys(lines)).toEqual([])
    }
    const { lines } = chartLines({ tab: 'blockers', chart: BLOCKERS_CHART, own: BLOCKERS, selected: null, unfolded: ['gate'] })
    expect(texts(lines).slice(2)).toEqual(['gate  claimed  holds up 2', '├─ c', '│  └─ d', 'host  claimed  holds up 1'])
  })

  test('a root the model no longer lists keeps its name for when it comes back', () => {
    const chart = ansiLines('blockers\nhost  claimed  holds up 1\n└─ e\n')
    const { lines } = chartLines({ tab: 'blockers', chart, own: blockersJson([root('host')]), selected: null, unfolded: ['gate'] })
    expect(texts(lines)).toEqual(['blockers', 'host  claimed  holds up 1'])
  })

  test('nothing blocked: both lines, none pressable', () => {
    const chart = ansiLines('commit abc\nblockers\nnothing is blocked\n')
    const { lines } = chartLines({ tab: 'blockers', chart, own: blockersJson([]), selected: null, unfolded: [] })
    expect(texts(lines)).toEqual(['commit abc', 'blockers', 'nothing is blocked'])
    expect(keys(lines)).toEqual([])
  })
})

describe('component-heading', () => {
  const chartWith = (heading: string) =>
    ansiLines(`commit abc1234\n${heading}\n  a  P5  ready\n  b  P5  ready\nchanges\n  none\n`)
  const two = flightJson([flightA, { ...flightA, node: 'b' }])

  for (const [name, heading] of [
    ['the new heading', 'components'],
    ['an older clm', 'nodes'],
  ] as const) {
    // req:plugin.panel/component-heading@50ce90a7
    test(name, () => {
      const { lines } = chartLines({ tab: 'flight', chart: chartWith(heading), own: two, selected: null, unfolded: [] })
      expect(keys(lines)).toEqual(['node:a', 'node:b'])
    })
  }
})
