// Wires the panel's rules (`../src/panel.ts`, `../src/agents.ts`) to `$`: `/clm:view`, the
// `CLM` pane and its redraws, and the build rows kept from the hooks events. The command
// itself is the `clm` plugin's `commands/view.md`; answering it here shows the output without
// a model turn.
import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register, RenderNode } from 'claude-code'

import {
  dismiss,
  flightRows,
  onComplete,
  onSessionEnd,
  onSpawn,
  onStep,
  onToolCall,
  onTurnStart,
  reconcile,
  reconcileWaiting,
  ticking,
} from '../src/agents.ts'
import { type PaneLine, keyTarget, modelRuns, paneChart, pressNode, toggleRoot, withBuildRows } from '../src/inspector.ts'
import { MACHINE_MS, machineLines, machineSampler } from '../src/machine.ts'
import {
  FIRST_TAB,
  ORIGIN_MAIN_ARGV,
  PANE_ID,
  PANE_TITLE,
  REDRAW_MS,
  TABS,
  type Run,
  type Span,
  type Surface,
  cachedRun,
  clmRoot,
  opensForSkill,
  command,
  type Fetched,
  fetchIfDue,
  fetchWarning,
  hasOriginMain,
  paneBody,
  runClm,
  tabOf,
  unfenced,
  unlessSuperseded,
  viewArgs,
  viewKey,
  widthOf,
} from '../src/panel.ts'

const tab = atom({ plugin: 'clm-panel', key: 'tab' } as const, FIRST_TAB, { shape: 'tab-1' })
// The node the inspector shows, and the Blockers tab's unfolded roots: only presses write them.
const selected = atom({ plugin: 'clm-panel', key: 'selected' } as const, null as string | null, { shape: 'selected-1' })
const unfolded = atom({ plugin: 'clm-panel', key: 'unfolded' } as const, [] as string[], { shape: 'unfolded-1' })
// Whether the inspector shows the node's contract: hidden until pressed, for every node.
const contract = atom({ plugin: 'clm-panel', key: 'contract' } as const, false, { shape: 'contract-1' })
// Bumped to redraw: the pane runs `clm view` again for each value.
const generation = atom({ plugin: 'clm-panel', key: 'generation' } as const, 0)
// The subagents this session launched, and the time their rows are drawn at.
const agents = atom({ plugin: 'clm-panel', key: 'agents' } as const, [])
const now = atom({ plugin: 'clm-panel', key: 'now' } as const, 0)
// Bumped when a `clm machine` sample finishes: the pane redraws from the views' cache.
const sampled = atom({ plugin: 'clm-panel', key: 'sampled' } as const, 0)

type PaneEvent = { surface: Surface; props: { bodyColumns: number } }

const LABELS = { queue: 'Queue', spec: 'Spec', flight: 'Flight', blockers: 'Blockers' } as const

let timer: { cancel(): void } | undefined
let tick: { cancel(): void } | undefined
let machineTimer: { cancel(): void } | undefined
// The columns the latest sample was drawn for: a pane resized draws its rows again at its width.
let sampledColumns: number | undefined
let columns = Number.NaN
let sampler: ReturnType<typeof machineSampler> | undefined
let lastFetch: number | undefined
// The latest fetch that finished, for the line drawn while it failed.
let latestFetch: Fetched | undefined
const runs = new Map<string, Promise<Run>>()
// Whether `refs/remotes/origin/main` exists, so every `clm view` passes `--at origin/main`:
// asked once per generation, so again after each fetch, which bumps it.
const atMain = new Map<string, Promise<boolean>>()

const isPlaced = async ($: EngineInterface) =>
  (await $.ui.panes()).some(p => p.id === PANE_ID && p.isPlaced)

// The engine's agents, or undefined when the call fails: the runs then stay as they were.
const listAgents = ($: EngineInterface) => $.agent.list().catch(() => undefined)

// The engine's status may lag the hooks: before each 30-second redraw, and at session start, the
// runs are checked against what it lists now (`waiting` ends, or begins again).
async function reconcileRuns($: EngineInterface) {
  const listed = await listAgents($)
  if (listed === undefined) return
  const at = await setNow($)
  await update($, agents, list => reconcileWaiting(list, listed, at))
}

async function redraw($: EngineInterface) {
  await update($, generation, n => n + 1)
  const cwd = await $.session.cwd()
  const fetch = fetchIfDue($, cwd, await $.clock.now(), lastFetch)
  lastFetch = fetch.lastFetch
  if (fetch.fetching) {
    latestFetch = (await fetch.fetching) ?? latestFetch
    await update($, generation, n => n + 1)
  }
}

// One `clm machine` run, unless the last is still going; the pane redraws without rerunning `clm view`.
async function sampleMachine($: EngineInterface) {
  sampler ??= machineSampler($, () => $.session.cwd())
  const width = widthOf(columns)
  if (await sampler.sample(columns)) {
    sampledColumns = width
    await update($, sampled, n => n + 1)
  }
}

function startTimer($: EngineInterface) {
  machineTimer?.cancel()
  machineTimer = $.clock.every(MACHINE_MS, () => {
    void isPlaced($).then(placed => (placed ? sampleMachine($) : undefined))
  })
  timer?.cancel()
  timer = $.clock.every(REDRAW_MS, () => {
    void isPlaced($).then(async placed => {
      if (!placed) return
      await reconcileRuns($)
      await redraw($)
    })
  })
}

function stopTimer() {
  timer?.cancel()
  timer = undefined
  machineTimer?.cancel()
  machineTimer = undefined
}

const setNow = async ($: EngineInterface) => {
  const at = await $.clock.now()
  await update($, now, () => at)
  return at
}

// Opens the pane on Queue, as `/clm:view` with no argument does.
async function openPane($: EngineInterface) {
  await update($, tab, () => FIRST_TAB)
  await $.ui.open({ id: PANE_ID, title: PANE_TITLE })
  startTimer($)
  void redraw($)
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    // A reload drops the timer, not the pane.
    if ((await $.ui.panes()).some(p => p.id === PANE_ID)) startTimer($)
    else stopTimer()
    const started = await next(e)
    // A reload keeps the runs; one the engine no longer lists will never finish.
    const listed = await $.agent.list()
    await update($, agents, list => reconcile(list, listed))
    await reconcileRuns($)
    // While a run is running or waiting the rows tick each second: `now` changes, `clm` doesn't run.
    tick?.cancel()
    tick = $.clock.every(1000, () => {
      read($, agents)
        .then(list => (ticking(list) ? setNow($) : undefined))
        .catch(() => undefined)
    })
    return started
  })

  on('session.end', async ($, e, next) => {
    const ended = await next(e)
    await update($, agents, list => onSessionEnd(list, e.reason))
    return ended
  })

  on('agent.spawn', async ($, e, next) => {
    const started = await next(e)
    const at = await setNow($)
    await update($, agents, list => onSpawn(list, e, started, at))
    return started
  })

  // Each model request of a subagent: its context, tokens, cost, model and effort.
  on('turn.step', async function* ($, e, next) {
    const result = yield* next(e)
    if (e.agentId) await update($, agents, list => onStep(list, e, result.usage))
    return result
  })

  // A subagent's task list, from its calls that succeed.
  on('tool.call', async ($, e, next) => {
    const ran = await next(e)
    if (e.agentId) await update($, agents, list => onToolCall(list, e, ran))
    return ran
  })

  on('command.run', { command: 'clm:view' }, async ($, e) => {
    const answer = await command($, e.args, await $.session.cwd())
    if ('text' in answer) return { text: answer.text }
    await openPane($)
    return { text: 'Opened the CLM panel.' }
  })

  // The build skill opens the pane on Queue when it isn't open; the prompt is left as it came.
  on('skill.prompt', async ($, e, next) => {
    const prompt = await next(e)
    // Other skills cost no lookup.
    if (!opensForSkill({ skill: e.skill, pluginName: e.pluginName, panes: [], inRepo: true })) return prompt
    try {
      const inRepo = (await clmRoot($, await $.session.cwd())) !== undefined
      if (opensForSkill({ skill: e.skill, pluginName: e.pluginName, panes: await $.ui.panes(), inRepo })) {
        await openPane($)
      }
    } catch {
      // A failure while opening leaves the prompt as it was.
    }
    return prompt
  })

  // The answer's row in the transcript, drawn as a code block.
  on('ui.render', { component: 'CommandOutput', props: { command: 'clm:view' } }, async ($, e, next) => {
    const source = e.props.isErrored ? undefined : unfenced(e.props.text)
    if (source === undefined) return next(e)
    const { Code } = $.ui.resolve(e)
    return <Code source={source} />
  })

  on('ui.close', async ($, e, next) => {
    if (e.id === PANE_ID) stopTimer()
    return next(e)
  })

  // A listed agent's loop starting a turn again (the engine resuming it when its background
  // work ends, or a message waking it): its run is running and drawn again.
  on('turn.start', async ($, e, next) => {
    const started = await next(e)
    if (e.agentId) await update($, agents, list => onTurnStart(list, e))
    return started
  })

  on('turn.complete', async ($, e, next) => {
    const result = await next(e)
    if (e.agentId) {
      // An answer while the engine lists the agent `waiting` leaves it waiting, else done.
      const listed = e.reason === 'answer' ? (await listAgents($))?.find(a => a.id === e.agentId)?.status : undefined
      const at = await setNow($)
      await update($, agents, list => onComplete(list, e, at, listed))
    }
    if (timer && (await isPlaced($))) void redraw($)
    return result
  })

  // A draw the engine aborted for a newer one ends here, drawing nothing; the newer one draws.
  on('ui.render', { component: 'Pane', requestId: PANE_ID }, ($, e) => unlessSuperseded(() => drawPane($, e), ''))
}

async function drawPane($: EngineInterface, e: PaneEvent): Promise<RenderNode> {
  // A tab kept by an earlier version of the panel that is none of the four reads as Queue.
  const shown = tabOf(await read($, tab))
  // The machine rows follow the latest sample; a pane measured at a new width samples again.
  await read($, sampled)
  columns = e.props.bodyColumns
  if (sampledColumns !== widthOf(columns)) void sampleMachine($).catch(() => undefined)
  // Each surface's table is one member of a union: `Svg` is read by narrowing it.
  const elements = $.ui.resolve(e)
  const { Box, Text, Button } = elements
  const Svg = 'Svg' in elements ? elements.Svg : undefined
  const gen = await read($, generation)
  // A draw's run the engine cut short is asked again, as `clm`'s runs are.
  const atOrigin = await cachedRun(atMain, viewKey(gen, ORIGIN_MAIN_ARGV), () =>
    $.session.cwd().then(cwd => hasOriginMain($, cwd)),
  )
  const args = viewArgs(shown, e.surface, e.props.bodyColumns, atOrigin)
  const clm = (argv: string[]) =>
    cachedRun(runs, viewKey(gen, argv), () => $.session.cwd().then(cwd => runClm($, argv, cwd)))
  // The JSON models beside an ANSI chart, so its lines can be pressed (`node-inspector`),
  // started with it.
  const chosen = await read($, selected)
  const models = modelRuns(shown, e.surface, chosen, atOrigin)
  const [run, own, other] = await Promise.all([
    clm(args),
    models.own && clm(models.own),
    models.other && clm(models.other),
  ])
  // The build rows go on the Flight tab only, drawn at the latest tick.
  // Only the Flight tab reads `now`, so a tick redraws no other tab.
  const list = shown === 'flight' ? await read($, agents) : []
  const at = shown === 'flight' ? Math.max(await read($, now), ...list.map(a => a.startedAt)) : 0

  // A span with `button` is drawn as a button after the line's text, not as text.
  const runsOf = (line: Span[]) =>
    line.length === 0
      ? ' '
      : line
          .filter(s => s.button === undefined)
          .map(s => (
            <Text color={s.color} bold={s.bold} dimColor={s.dim}>
              {s.text}
            </Text>
          ))
  const machine = machineLines(sampler?.state() ?? {}).map(line => <Text wrap="truncate-end">{runsOf(line)}</Text>)
  // A press changes an atom and nothing else: the redraw reruns no `clm` it already holds.
  const press = (key: string) => {
    const target = keyTarget(key)
    if (target && 'node' in target) void update($, selected, s => pressNode(s, target.node))
    else if (target && 'root' in target) void update($, unfolded, u => toggleRoot(u, target.root))
    else if (target && 'dismiss' in target) void update($, agents, list => dismiss(list, target.dismiss))
  }
  const lines = (drawn: PaneLine[], wrap: 'wrap' | 'truncate-end' = 'truncate-end') =>
    drawn.map(line => {
      const buttons = line.spans.filter(s => s.button !== undefined)
      if (buttons.length > 0) {
        // A failed build row's first line: its text, then `Dismiss`.
        return (
          <Box flexDirection="row" gap={1}>
            <Text wrap={wrap}>{runsOf(line.spans)}</Text>
            {buttons.map(b => (
              <Button key={b.button} label={b.text} role="dismiss" onPress={() => press(b.button!)} />
            ))}
          </Box>
        )
      }
      return line.key === undefined ? (
        <Text wrap={wrap}>{runsOf(line.spans)}</Text>
      ) : (
        <Button key={line.key} plain onPress={() => press(line.key!)}>
          <Text wrap={wrap}>{runsOf(line.spans)}</Text>
        </Button>
      )
    })
  const tabs = (
    <Box flexDirection="row" gap={1}>
      {TABS.map(t => (
        <Button
          key={t}
          label={LABELS[t]}
          variant={t === shown ? 'primary' : 'secondary'}
          onPress={() => {
            // A tab change redraws: `clm view` runs again even for a tab shown before.
            void update($, tab, () => t).then(() => update($, generation, n => n + 1))
          }}
        />
      ))}
    </Box>
  )
  const body = paneBody(e.surface, run, shown)
  // Above the chart while the latest fetch failed, until one succeeds.
  const warning = fetchWarning(latestFetch, await $.clock.now())
  const fetchLine = warning && <Text wrap="truncate-end">{runsOf(warning)}</Text>
  if ('error' in body) {
    return (
      <Box flexDirection="column">
        {tabs}
        {machine}
        {fetchLine}
        <Text color="error">{body.error}</Text>
      </Box>
    )
  }
  // An ANSI chart carries the build rows under its node lines; the SVG is followed by them.
  const drawn =
    'ansi' in body ? undefined
    : 'text' in body ? body.text.replace(/\n$/, '').split('\n').map(line => <Text wrap="truncate-end">{line === '' ? ' ' : line}</Text>)
    : 'svg' in body && Svg ? <Svg source={body.svg} alt={`clm view ${shown}`} />
    : undefined
  if (!('ansi' in body)) {
    return (
      <Box flexDirection="column">
        {tabs}
        {machine}
        {fetchLine}
        {drawn}
        {lines(flightRows(undefined, list, at).map(spans => ({ spans })))}
      </Box>
    )
  }
  // Pressable node and root lines, folded chains and the inspector, or the chart as it came.
  const chart = paneChart({
    tab: shown,
    chart: body.ansi,
    own,
    other,
    selected: chosen,
    unfolded: await read($, unfolded),
    columns: e.props.bodyColumns,
    contract: await read($, contract),
  })
  const [head, ...facts] = chart.inspector ?? []
  const { detail } = chart
  return (
    <Box flexDirection="column">
      {tabs}
      {machine}
      {fetchLine}
      {head && (
        <Box flexDirection="row" gap={2}>
          <Text wrap="truncate-end">{runsOf(head)}</Text>
          <Button key="close" label="Close" role="dismiss" onPress={() => void update($, selected, () => null)} />
        </Box>
      )}
      {facts.map(line => (
        <Text wrap="truncate-end">{runsOf(line)}</Text>
      ))}
      {detail?.lines.map(line => (
        <Text wrap="truncate-end">{runsOf(line)}</Text>
      ))}
      {detail && (
        <Button
          key="contract"
          label="Contract"
          variant={detail.button}
          onPress={() => void update($, contract, was => !was)}
        />
      )}
      {detail?.contract?.map(line => <Text>{line === '' ? ' ' : line}</Text>)}
      {lines(shown === 'flight' ? withBuildRows(chart.lines, list, at) : chart.lines)}
    </Box>
  )
}
