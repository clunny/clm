// req:plugin.panel/build-rows@2a02f823
import ts from 'typescript'
import { describe, expect, test } from 'vitest'

import {
  type AgentRun,
  type Usage,
  applyComplete,
  applyStep,
  applyTool,
  bar,
  buildNode,
  dismiss,
  costOf,
  flightLines,
  flightRows,
  fmtCost,
  fmtTime,
  fmtTokens,
  lineText,
  modelName,
  onComplete,
  onTurnStart,
  reconcileWaiting,
  onSessionEnd,
  onSpawn,
  onStep,
  onToolCall,
  PRICES_CHECKED,
  ticking,
  progress,
  reconcile,
  resetsOn,
  startRun,
  svgLines,
  windowOf,
} from '../src/agents.ts'
import { keyTarget } from '../src/inspector.ts'
import { type Host, type Run, type Span, ansiLines, cachedRun, command, viewKey } from '../src/panel.ts'

const PROMPT = (node: string) => `Run \`git switch impl/${node}\` and check that it prints.\nMore.`

/** Starts runs in order, a second apart, from `[id, description, prompt, parentId?]`. */
function started(...specs: [string, string, string, string?][]): AgentRun[] {
  return specs.reduce<AgentRun[]>(
    (list, [id, description, prompt, parentId], i) =>
      startRun(list, { id, parentId, type: 'general-purpose', description, model: 'claude-opus-5-5', prompt, at: i * 1000 }),
    [],
  )
}

describe('runs', () => {
  test('a build is named by the first of its first five lines that names a step, never its description', () => {
    expect(buildNode(PROMPT('git'))).toBe('git')
    expect(buildNode(PROMPT('plugin.panel'))).toBe('plugin.panel')
    expect(buildNode('Review this.\nRun `git switch impl/git` first.')).toBe('git')
    expect(buildNode('a\nb\nc\nd\ne\nRun `git switch impl/git` first.')).toBeUndefined()
    expect(buildNode('x `git switch impl/a`\ny `git switch impl/b`')).toBe('a')
    expect(buildNode('build git')).toBeUndefined()
    const [run] = started(['a1', 'build git', 'Look around.'])
    expect(run?.node).toBeUndefined()
    expect(started(['a1', 'anything', PROMPT('git')])[0]?.node).toBe('git')
  })

  test('a run records what the spawn said, running from its start', () => {
    const [run] = started(['a1', 'code review', 'Review.', 'b0'])
    expect(run).toMatchObject({
      id: 'a1',
      parentId: 'b0',
      type: 'general-purpose',
      description: 'code review',
      model: 'claude-opus-5-5',
      status: 'running',
      startedAt: 0,
      round: 1,
      tokens: 0,
      steps: 0,
      tasks: [],
    })
  })

  test('a description a listed run already has is the next round', () => {
    const list = started(['a1', 'build git', PROMPT('git')], ['a2', 'Build  Git!', PROMPT('git')], ['a3', 'build init', PROMPT('init')])
    expect(list.map(r => r.round)).toEqual([1, 2, 1])
  })

  test('a spawn that resumes a listed agent keeps its run, running again', () => {
    let list = started(['b1', 'build git', PROMPT('git')])
    list = applyTool(list, 'b1', 'TodoWrite', { todos: [{ content: 'a', status: 'completed' }] })
    list = applyComplete(list, 'b1', true, 5_000, usage(1_000))
    list = startRun(list, { id: 'b1', type: 'general-purpose', description: 'build git', model: 'claude-opus-5-5', prompt: 'Carry on.', at: 9_000 })
    expect(list).toHaveLength(1)
    expect(list[0]).toMatchObject({ node: 'git', status: 'running', startedAt: 0, round: 1, tokens: 1_000 })
    expect(list[0]?.endedAt).toBeUndefined()
    expect(list[0]?.tasks).toHaveLength(1)
  })

  test('the list keeps the last 200 runs', () => {
    const list = started(...Array.from({ length: 201 }, (_, i): [string, string, string] => [`a${i}`, `run ${i}`, 'x']))
    expect(list).toHaveLength(200)
    expect(list[0]?.id).toBe('a1')
  })

  test('clear and resume empty the list; quitting does not', () => {
    expect(resetsOn('clear')).toBe(true)
    expect(resetsOn('resume')).toBe(true)
    expect(resetsOn('prompt_input_exit')).toBe(false)
  })

  test('after a reload a running run the engine no longer lists is dropped', () => {
    const list = started(['a1', 'one', 'x'], ['a2', 'two', 'x'], ['a3', 'three', 'x']).map(r =>
      r.id === 'a2' ? { ...r, status: 'done' as const } : r,
    )
    expect(reconcile(list, [{ id: 'a3' }]).map(r => r.id)).toEqual(['a2', 'a3'])
    const waiting = list.map(r => (r.id === 'a1' ? { ...r, status: 'waiting' as const } : r))
    expect(reconcile(waiting, [{ id: 'a3' }]).map(r => r.id)).toEqual(['a2', 'a3'])
  })
})

const usage = (input: number, output = 0, read = 0, write = 0): Usage => ({
  input_tokens: input,
  output_tokens: output,
  cache_read_input_tokens: read,
  cache_creation_input_tokens: write,
})

describe('updates', () => {
  test('each model request sets the context and adds to tokens, cost and steps', () => {
    let list = started(['a1', 'build git', PROMPT('git')])
    list = applyStep(list, 'a1', 'claude-opus-5-5', 'high', usage(100_000, 1_000, 50_000, 0))
    list = applyStep(list, 'a1', 'claude-opus-5-5', undefined, usage(10_000, 2_000, 0, 0))
    list = applyStep(list, 'other', 'claude-opus-5-5', 'low', usage(1))
    expect(list[0]).toMatchObject({ contextTokens: 12_000, tokens: 163_000, steps: 2, effort: 'high', model: 'claude-opus-5-5' })
    // 0.1M in at $4, 0.001M out at $20, 0.05M read at $0.2, then 0.01M in at $4 and 0.002M out.
    expect(list[0]?.costUsd).toBeCloseTo(0.4 + 0.02 + 0.01 + 0.04 + 0.04)
  })

  test('the turn ends the run: done on an answer, else failed, at that time', () => {
    const list = started(['a1', 'one', 'x'], ['a2', 'two', 'x'])
    const done = applyComplete(applyStep(list, 'a1', 'claude-opus-5-5', undefined, usage(1_000)), 'a1', true, 5_000)
    expect(done[0]).toMatchObject({ status: 'done', endedAt: 5_000, tokens: 1_000 })
    expect(applyComplete(list, 'a2', false, 7_000)[1]).toMatchObject({ status: 'failed', endedAt: 7_000 })
  })

  test("a run that saw no step takes the turn's own usage", () => {
    const list = applyComplete(started(['a1', 'one', 'x']), 'a1', true, 1_000, { ...usage(1_000_000), model: 'claude-haiku-4-5' })
    expect(list[0]).toMatchObject({ tokens: 1_000_000, model: 'claude-haiku-4-5' })
    expect(list[0]?.costUsd).toBeCloseTo(1)
  })

  test('TodoWrite replaces the task list; k, n and the current step follow it', () => {
    const todos = [
      { content: 'one', status: 'completed' },
      { content: 'two', status: 'completed' },
      { content: 'three', status: 'pending' },
      { content: 'four', status: 'in_progress' },
      { content: 'five', status: 'pending' },
    ]
    const list = applyTool(started(['a1', 'one', 'x']), 'a1', 'TodoWrite', { todos })
    expect(progress(list[0]?.tasks ?? [])).toEqual({ k: 2, n: 5, current: 'four' })
    expect(progress([])).toEqual({ k: 0, n: 0 })
  })

  test('TaskCreate appends a pending task; TaskUpdate changes or deletes one', () => {
    let list = started(['a1', 'one', 'x'])
    list = applyTool(list, 'a1', 'TaskCreate', { subject: 'write tests' }, '1')
    list = applyTool(list, 'a1', 'TaskCreate', { subject: 'implement' }, '2')
    expect(progress(list[0]?.tasks ?? [])).toEqual({ k: 0, n: 2, current: 'write tests' })
    list = applyTool(list, 'a1', 'TaskUpdate', { taskId: '1', status: 'completed' })
    list = applyTool(list, 'a1', 'TaskUpdate', { taskId: '2', status: 'in_progress', subject: 'implement it' })
    expect(progress(list[0]?.tasks ?? [])).toEqual({ k: 1, n: 2, current: 'implement it' })
    list = applyTool(list, 'a1', 'TaskUpdate', { taskId: '1', status: 'deleted' })
    expect(progress(list[0]?.tasks ?? [])).toEqual({ k: 0, n: 1, current: 'implement it' })
    // Another tool's input holds no task field.
    expect(applyTool(list, 'a1', 'Read', {})).toEqual(list)
  })
})

describe('cost and formatting', () => {
  test('cost is priced by the most specific row, with the sonnet row for an unknown model', () => {
    // The rows of `effort`'s `prices.rs`, checked the same day.
    expect(PRICES_CHECKED).toBe('2026-10-30')
    const million = usage(1_000_000)
    const reads = usage(0, 0, 1_000_000)
    expect(costOf('claude-fable-5-1', million)).toBe(10)
    expect(costOf('claude-fable-5-1', reads)).toBe(0.25)
    expect(costOf('claude-mythos-5-1', reads)).toBe(0.25)
    expect(costOf('claude-fable-5', reads)).toBe(1)
    expect(costOf('claude-opus-5-5', million)).toBe(4)
    expect(costOf('claude-opus-5-5', reads)).toBe(0.2)
    expect(costOf('claude-opus-5', million)).toBe(5)
    expect(costOf('claude-opus-4-6', million)).toBe(5)
    expect(costOf('claude-opus-4-9', million)).toBe(15)
    expect(costOf('claude-haiku-5-5', usage(1_000_000, 1_000_000, 1_000_000, 1_000_000))).toBeCloseTo(0.1 + 0.5 + 0.01 + 0.125)
    expect(costOf('claude-opus-4-1', million)).toBe(15)
    expect(costOf('claude-sonnet-5-0', million)).toBe(2)
    expect(costOf('claude-sonnet-4-5', million)).toBe(3)
    expect(costOf('claude-haiku-4-5', usage(0, 1_000_000, 1_000_000, 1_000_000))).toBeCloseTo(5 + 0.1 + 1.25)
    expect(costOf('some-other-model', usage(0, 1_000_000))).toBe(15)
  })

  test("the rows price every model as effort's prices.rs does", () => {
    // Read through the compiler's `ts.sys`, as the type-check tests read files: no Node here.
    const source = ts.sys.readFile(`${ts.sys.getCurrentDirectory()}/../crates/clm-effort/src/prices.rs`)
    if (source === undefined) throw new Error('no prices.rs')
    const rates = (list: string) => list.split(',').map(Number)
    const rows = [...source.matchAll(/\("([a-z0-9-]+)", \[([^\]]+)\]\)/g)].map(m => [m[1]!, rates(m[2]!)] as const)
    const fallback = /const SONNET: Rates = \[([^\]]+)\]/.exec(source)?.[1]
    expect(rows.length).toBeGreaterThan(5)
    expect(fallback).toBeDefined()
    // Each row's pattern as a model ID, then one no row matches.
    for (const [model, want] of [...rows, ['some-other-model', rates(fallback!)] as const]) {
      const each = [usage(1e6), usage(0, 1e6), usage(0, 0, 1e6), usage(0, 0, 0, 1e6)].map(u => costOf(`claude-${model}`, u))
      expect([model, ...each]).toEqual([model, ...want])
    }
  })

  test('the context window is 200k for Haiku and 1M otherwise', () => {
    expect(windowOf('claude-haiku-4-5')).toBe(200_000)
    expect(windowOf('claude-opus-5-5')).toBe(1_000_000)
  })

  test('model names, tokens, cost and time read as the rows show them', () => {
    expect(modelName('claude-fable-5-1')).toBe('Fable 5.1')
    expect(modelName('claude-opus-5-5[1m]')).toBe('Opus 5.5')
    expect(modelName('claude-haiku-4-5-20251001')).toBe('Haiku 4.5')
    expect(modelName('mystery')).toBe('mystery')
    expect([850, 85_000, 1_300_000].map(fmtTokens)).toEqual(['850', '85k', '1.3M'])
    expect([999.6, 999_600, 1_000_000].map(fmtTokens)).toEqual(['1k', '1.0M', '1.0M'])
    expect([4, 9.996, 12.34].map(fmtCost)).toEqual(['$4.00', '$10.0', '$12.3'])
    expect([65_000, 3_725_000].map(fmtTime)).toEqual(['1:05', '1:02:05'])
    expect(bar(0.25)).toBe('███░░░░░░░')
    expect(bar(2)).toBe('██████████')
  })
})

/** A flight chart as `clm view flight --format ansi` draws it, cut to `width`-ish node IDs. */
const CHART = ansiLines(
  [
    '\x1b[34mflight\x1b[0m  2 nodes pending',
    '\x1b[34mnodes\x1b[0m',
    '  git    P5  \x1b[33mclaimed\x1b[0m  ░ 1 pending',
    '  init   P5  \x1b[36mready\x1b[0m  ░ 1 pending',
    '\x1b[34mchanges\x1b[0m',
    '  applied',
    '    20261025-x  P5  ███░ 1/2',
    '',
  ].join('\n'),
)

const text = (lines: Span[][]) => lines.map(l => l.map(s => s.text).join(''))

/** The rows between the line starting `from` and the next line starting `to`. */
function between(lines: string[], from: string, to: string): string[] {
  const start = lines.findIndex(l => l.startsWith(from))
  const end = lines.findIndex((l, i) => i > start && l.startsWith(to))
  expect(start).toBeGreaterThanOrEqual(0)
  expect(end).toBeGreaterThan(start)
  return lines.slice(start + 1, end)
}

/** The running build of `git` of the first scenario, a minute in. */
function gitBuild(): AgentRun[] {
  let list = started(['b1', 'build git', PROMPT('git')])
  list = applyStep(list, 'b1', 'claude-opus-5-5', 'high', usage(50_000, 1_000))
  list = applyStep(list, 'b1', 'claude-opus-5-5', 'high', usage(60_000, 2_000))
  return applyTool(list, 'b1', 'TodoWrite', {
    todos: [
      { content: 'cite markers', status: 'completed' },
      { content: 'record runs', status: 'completed' },
      { content: 'draw the rows', status: 'in_progress' },
      { content: 'wire it', status: 'pending' },
      { content: 'review', status: 'pending' },
    ],
  })
}

const glyphs = (lines: Span[][]) => lines.flat().filter(s => ['●', '✓', '✗'].includes(s.text))

describe('drawing', () => {
  test('a build is drawn under its node line, with its facts', () => {
    const rows = between(text(flightLines(CHART, gitBuild(), 61_000)), '  git ', '  init ')
    const joined = rows.join('\n')
    expect(rows).toHaveLength(3)
    expect(rows[0]).toMatch(/●.*build git/)
    expect(rows[1]).toContain('general-purpose · Opus 5.5 · high')
    expect(rows[2]).toContain('████░░░░░░ 2/5')
    expect(joined).toContain('2/5 draw the rows')
    expect(joined).toMatch(/ctx \d+% · /)
    expect(joined).toContain('tok ≈$')
    expect(joined).toContain('1:01')
    expect(glyphs(flightLines(CHART, gitBuild(), 61_000))).toEqual([{ text: '●', color: 'yellow' }])
  })

  test('a nested subagent follows its build, indented and prefixed └, before the next node', () => {
    const list = startRun(gitBuild(), { id: 'c1', parentId: 'b1', type: 'code-review', description: 'code review', model: 'claude-sonnet-5-0', prompt: 'Review.', at: 30_000 })
    const rows = between(text(flightLines(CHART, list, 61_000)), '  git ', '  init ')
    expect(rows).toHaveLength(6)
    expect(rows[3]).toMatch(/^ +└ ● code review$/)
    const indent = (s: string) => s.length - s.trimStart().length
    expect(indent(rows[3] ?? '')).toBeGreaterThan(indent(rows[0] ?? ''))
    expect(rows[5]).toContain('0:31')
  })

  test('children follow oldest first, indented two cells a level up to four levels', () => {
    let list = gitBuild()
    const child = (id: string, parentId: string, description: string, at: number) =>
      (list = startRun(list, { id, parentId, type: 'general-purpose', description, model: 'claude-opus-5-5', prompt: 'Go.', at }))
    child('c2', 'b1', 'second child', 20_000)
    child('c1', 'b1', 'first child', 10_000)
    child('d1', 'c2', 'depth 2', 21_000)
    child('d2', 'd1', 'depth 3', 22_000)
    child('d3', 'd2', 'depth 4', 23_000)
    child('d4', 'd3', 'depth 5', 24_000)
    const rows = between(text(flightLines(CHART, list, 61_000)), '  git ', '  init ')
    const firsts = rows.filter((_, i) => i % 3 === 0)
    expect(firsts.map(l => l.replace(/^.*[●✓✗] /, ''))).toEqual([
      'build git',
      'first child',
      'second child',
      'depth 2',
      'depth 3',
      'depth 4',
      'depth 5',
    ])
    const indent = (s: string) => s.length - s.trimStart().length
    const base = indent(firsts[0] ?? '')
    expect(firsts.map(l => indent(l) - base)).toEqual([0, 2, 2, 4, 6, 8, 8])
  })

  test('the totals name the most used model first', () => {
    let list = startRun(gitBuild(), { id: 'x1', type: 'Explore', description: 'look one', model: 'claude-haiku-4-5', prompt: 'Look.', at: 10_000 })
    list = startRun(list, { id: 'x2', type: 'Explore', description: 'look two', model: 'claude-haiku-4-5', prompt: 'Look.', at: 11_000 })
    expect(text(flightLines(CHART, list, 61_000)).at(-1)).toMatch(/· Haiku 4\.5 ×2 · Opus 5\.5 ×1$/)
  })

  // req:plugin.panel/build-row-lifecycle@2de7d406
  test('a finished build and its subagents leave the rows at once; the totals still count them', () => {
    let list = startRun(gitBuild(), { id: 'c1', parentId: 'b1', type: 'code-review', description: 'code review', model: 'claude-opus-5-5', prompt: 'Review.', at: 30_000 })
    list = applyStep(list, 'c1', 'claude-opus-5-5', undefined, usage(10_000))
    list = applyComplete(list, 'c1', true, 80_000)
    list = applyComplete(list, 'b1', true, 90_000)
    const all = text(flightLines(CHART, list, 90_000))
    expect(all.join('\n')).not.toContain('build git')
    expect(all.join('\n')).not.toContain('code review')
    expect(all[all.findIndex(l => l.startsWith('  git ')) + 1]).toMatch(/^ {2}init /)
    expect(all).not.toContain('agents')
    // 113k of the build and 10k of the review, both on Opus 5.5.
    expect(all.at(-1)).toMatch(/^≈\$\d+\.\d\d · 123k tokens · 1:30 · Opus 5\.5 ×2$/)
    expect(glyphs(flightLines(CHART, list, 0))).toEqual([])
  })

  test('a failed build shows ✗ in rose and a Dismiss button, until the button is pressed', () => {
    const list = applyComplete(gitBuild(), 'b1', false, 90_000)
    const lines = flightLines(CHART, list, 0)
    expect(glyphs(lines)).toEqual([{ text: '✗', color: 'red' }])
    const first = lines.find(l => lineText(l).includes('build git'))
    expect(first?.at(-1)).toEqual({ text: 'Dismiss', button: 'dismiss:b1' })
    expect(keyTarget('dismiss:b1')).toEqual({ dismiss: 'b1' })
    const gone = dismiss(list, 'b1')
    const after = text(flightLines(CHART, gone, 0))
    expect(after.join('\n')).not.toContain('build git')
    expect(after.at(-1)).toMatch(/^≈\$\d+\.\d\d · 113k tokens · 1:30 · Opus 5\.5 ×1$/)
    // A run the engine resumes is running again, and drawn again.
    const resumed = startRun(gone, { id: 'b1', type: 'general-purpose', description: 'build git', model: 'claude-opus-5-5', prompt: 'Go on.', at: 95_000 })
    expect(glyphs(flightLines(CHART, resumed, 95_000))).toEqual([{ text: '●', color: 'yellow' }])
  })

  test("a failed child of a finished build takes its parent's place, at the top level under its node", () => {
    let list = startRun(gitBuild(), { id: 'c1', parentId: 'b1', type: 'code-review', description: 'code review', model: 'claude-opus-5-5', prompt: 'Review.', at: 30_000 })
    list = applyComplete(list, 'c1', false, 80_000)
    list = applyComplete(list, 'b1', true, 90_000)
    const rows = between(text(flightLines(CHART, list, 90_000)), '  git ', '  init ')
    expect(rows).toHaveLength(3)
    expect(rows[0]).toMatch(/^ {4}✗ code review/)
    expect(rows[0]).not.toContain('└')
    // Under a running grandparent, it sits under that one instead.
    let deep = startRun(gitBuild(), { id: 'c1', parentId: 'b1', type: 'x', description: 'middle', model: 'm', prompt: 'x', at: 30_000 })
    deep = startRun(deep, { id: 'd1', parentId: 'c1', type: 'x', description: 'leaf', model: 'm', prompt: 'x', at: 31_000 })
    deep = applyComplete(applyComplete(deep, 'd1', false, 40_000), 'c1', true, 41_000)
    const under = between(text(flightLines(CHART, deep, 61_000)), '  git ', '  init ')
    expect(under[0]).toMatch(/● build git/)
    expect(under[3]).toMatch(/^ {6}└ ✗ leaf/)
  })

  test('clearing the session draws no build row and no totals', () => {
    const list = applyComplete(gitBuild(), 'b1', false, 90_000)
    expect(flightLines(CHART, onSessionEnd(list, 'clear'), 0)).toEqual(CHART)
    expect(svgLines(onSessionEnd(list, 'clear'), 0)).toEqual([])
  })

  test('a subagent that is not a build follows the chart under agents while it runs, and the totals count it', () => {
    let list = startRun(gitBuild(), { id: 'x1', type: 'Explore', description: 'look around', model: 'claude-haiku-4-5', prompt: 'Look.', at: 10_000 })
    list = applyStep(list, 'x1', 'claude-haiku-4-5', undefined, usage(1_000))
    const lines = flightLines(CHART, list, 61_000)
    const all = text(lines)
    const heading = all.indexOf('agents')
    expect(heading).toBeGreaterThan(all.findIndex(l => l.startsWith('    20261025-x')))
    expect(lines[heading]).toEqual([{ text: 'agents', color: 'blue' }])
    expect(all[heading + 1]).toMatch(/● look around/)
    expect(between(all, '  git ', '  init ').join('\n')).not.toContain('look around')
    expect(all.at(-1)).toMatch(/^≈\$\d+\.\d\d · 114k tokens · 1:01 · Opus 5\.5 ×1 · Haiku 4\.5 ×1$/)
    const done = text(flightLines(CHART, applyComplete(list, 'x1', true, 20_000), 61_000))
    expect(done).not.toContain('agents')
    expect(done.join('\n')).not.toContain('look around')
    expect(done.at(-1)).toMatch(/^≈\$\d+\.\d\d · 114k tokens · 1:01 · Opus 5\.5 ×1 · Haiku 4\.5 ×1$/)
  })

  test('a later round of a build sits above the first under its node', () => {
    let list = applyComplete(gitBuild(), 'b1', false, 20_000)
    list = startRun(list, { id: 'b2', type: 'general-purpose', description: 'build git', model: 'claude-opus-5-5', prompt: PROMPT('git'), at: 30_000 })
    const rows = between(text(flightLines(CHART, list, 61_000)), '  git ', '  init ')
    expect(rows[0]).toMatch(/● build git/)
    expect(rows[1]).toContain('round 2')
    expect(rows[3]).toMatch(/✗ build git/)
    expect(rows[4]).not.toContain('round')
  })

  // req:plugin.panel/build-row-cost@5e68018b
  test('a second later the rows show a larger elapsed time, and clm has not run again', async () => {
    const at = (ms: number) => text(flightLines(CHART, gitBuild(), ms)).join('\n')
    expect(at(61_000)).toContain('1:01')
    expect(at(62_000)).toContain('1:02')
    const cache = new Map<string, Promise<Run>>()
    let runs = 0
    const start = async (): Promise<Run> => ({ exitCode: 0, stdout: `${++runs}`, stderr: '' })
    const key = viewKey(3, ['view', 'flight', '--format', 'ansi', '--width', '80'])
    await cachedRun(cache, key, start)
    expect(await cachedRun(cache, key, start)).toMatchObject({ stdout: '1' })
    expect(runs).toBe(1)
    await cachedRun(cache, viewKey(4, ['view', 'flight']), start)
    expect(runs).toBe(2)
  })

  test("a draw waiting on another draw's run that rejects starts its own, and caches no failure", async () => {
    const cache = new Map<string, Promise<Run>>()
    const key = viewKey(5, ['view', 'flight'])
    const lost = cachedRun(cache, key, () => Promise.reject(new Error('aborted')))
    const waiting = cachedRun(cache, key, async () => ({ exitCode: 0, stdout: 'own', stderr: '' }))
    await expect(lost).rejects.toThrow('aborted')
    expect(await waiting).toMatchObject({ stdout: 'own' })
    expect(await cache.get(key)).toMatchObject({ stdout: 'own' })
  })

  test('draws whose every run rejects each fail after one start of their own', async () => {
    const cache = new Map<string, Promise<Run>>()
    const key = viewKey(6, ['view', 'flight'])
    let starts = 0
    const start = () => (starts++, Promise.reject(new Error('aborted')))
    const draws = [1, 2, 3].map(() => cachedRun(cache, key, start))
    for (const draw of draws) await expect(draw).rejects.toThrow('aborted')
    expect(starts).toBeLessThanOrEqual(3)
    expect(cache.has(key)).toBe(false)
  })

  test("a new generation's run drops the older generations' outputs", async () => {
    const cache = new Map<string, Promise<Run>>()
    const start = async (): Promise<Run> => ({ exitCode: 0, stdout: 'x', stderr: '' })
    await cachedRun(cache, viewKey(1, ['view', 'flight']), start)
    await cachedRun(cache, viewKey(1, ['view', 'spec']), start)
    await cachedRun(cache, viewKey(2, ['view', 'flight']), start)
    expect([...cache.keys()]).toEqual([viewKey(2, ['view', 'flight'])])
  })

  // req:plugin.panel/build-row-cost@5e68018b
  test('a million input tokens cost the model\'s input rate', () => {
    const list = applyStep(started(['b1', 'build git', PROMPT('git')]), 'b1', 'claude-opus-5-5', undefined, usage(1_000_000))
    const row = between(text(flightLines(CHART, list, 0)), '  git ', '  init ').join('\n')
    expect(row).toContain('1.0M tok ≈$4.00')
    expect(row).toContain('ctx 100%')
  })

  test('a node ID the chart cut with … still finds its builds', () => {
    const cut = ansiLines('nodes\n  plugin.p…  P5  claimed\n  plugin     P5  blocked\nchanges\n')
    const list = startRun([], { id: 'b1', type: 'general-purpose', description: 'build panel', model: 'm', prompt: PROMPT('plugin.panel'), at: 0 })
    expect(between(text(flightLines(cut, list, 0)), '  plugin.p…', '  plugin ')[0]).toMatch(/build panel/)
  })

  test('a build of a node the chart no longer lists goes under agents', () => {
    const list = startRun([], { id: 'b1', type: 'general-purpose', description: 'build web', model: 'm', prompt: PROMPT('web'), at: 0 })
    const all = text(flightLines(CHART, list, 0))
    expect(all[all.indexOf('agents') + 1]).toMatch(/build web/)
  })

  test('a run whose parent dropped off the list is drawn at the top level', () => {
    const list = startRun([], { id: 'c1', parentId: 'gone', type: 'x', description: 'orphan', model: 'm', prompt: 'x', at: 0 })
    const all = text(flightLines(CHART, list, 0))
    expect(all[all.indexOf('agents') + 1]).toMatch(/^ +● orphan$/)
  })

  test('on a surface that draws the SVG, the rows follow it grouped under node lines', () => {
    let list = startRun(gitBuild(), { id: 'x1', type: 'Explore', description: 'look around', model: 'claude-haiku-4-5', prompt: 'Look.', at: 10_000 })
    list = startRun(list, { id: 'i1', type: 'general-purpose', description: 'build init', model: 'claude-opus-5-5', prompt: PROMPT('init'), at: 20_000 })
    const all = text(svgLines(list, 61_000))
    expect(all[0]).toBe('git')
    expect(all[1]).toMatch(/● build git/)
    expect(all[4]).toBe('init')
    expect(all[5]).toMatch(/● build init/)
    expect(all[8]).toBe('agents')
    expect(all[9]).toMatch(/● look around/)
    expect(all.at(-1)).toMatch(/^≈\$/)
    expect(svgLines([], 0)).toEqual([])
  })

  test('after the SVG, only drawn runs follow, a failed child of a finished build under its node', () => {
    let list = startRun(gitBuild(), { id: 'c1', parentId: 'b1', type: 'code-review', description: 'code review', model: 'm', prompt: 'x', at: 30_000 })
    list = startRun(list, { id: 'x1', type: 'Explore', description: 'look around', model: 'm', prompt: 'x', at: 31_000 })
    list = applyComplete(applyComplete(applyComplete(list, 'c1', false, 40_000), 'b1', true, 50_000), 'x1', true, 50_000)
    const all = text(svgLines(list, 61_000))
    expect(all[0]).toBe('git')
    expect(all[1]).toMatch(/^ {2}✗ code review/)
    expect(all).not.toContain('agents')
    expect(all).toHaveLength(5)
    expect(all.at(-1)).toMatch(/^≈\$.* · m ×2 · Opus 5\.5 ×1$/)
  })

  test('/clm:view flight in the chat is clm\'s output alone, whatever runs', async () => {
    const files = ['/work/repo/specs/SPEC.md', '/plugins/clm/plugin/bin/clm']
    const $: Host = {
      plugin: { root: '/plugins/clm/plugin-panel' },
      process: { run: async () => ({ exitCode: 0, stdout: 'nodes\n  git  P5\nchanges\n', stderr: '' }) },
      fs: {
        stat: async path => (files.includes(path) ? { kind: 'file' } : Promise.reject(new Error(path))),
        list: async () => [],
      },
    }
    expect(await command($, 'flight', '/work/repo')).toEqual({ text: '\n```\nnodes\n  git  P5\nchanges\n```' })
  })
})

describe('hooks events', () => {
  const spawn = { prompt: PROMPT('git'), description: 'build git', subagentType: 'general-purpose' }

  test('agent.spawn records a started run from the event and what next answered', () => {
    const list = onSpawn([], { ...spawn, parentAgentId: 'p0' }, { agentId: 'b1', model: 'claude-opus-5-5' }, 5)
    expect(list[0]).toMatchObject({ id: 'b1', parentId: 'p0', type: 'general-purpose', node: 'git', model: 'claude-opus-5-5', startedAt: 5 })
    expect(onSpawn([], spawn, { deny: 'no' }, 0)).toEqual([])
    expect(onSpawn([], spawn, { model: 'm' }, 0)).toEqual([])
  })

  test('turn.step follows the request on its agent only; its usage model wins', () => {
    const list = onSpawn([], spawn, { agentId: 'b1', model: 'claude-opus-5-5' }, 0)
    const stepped = onStep(list, { agentId: 'b1', model: 'claude-opus-5-5', effort: 'high' }, { ...usage(1_000), model: 'claude-sonnet-5-0' })
    expect(stepped[0]).toMatchObject({ steps: 1, model: 'claude-sonnet-5-0', effort: 'high', tokens: 1_000 })
    expect(onStep(list, { model: 'm' }, usage(1))).toEqual(list)
    expect(onStep(list, { agentId: 'b1', model: 'm' }, undefined)).toEqual(list)
  })

  test('turn.complete ends the run: done on reason answer, failed otherwise', () => {
    const list = onSpawn([], spawn, { agentId: 'b1', model: 'm' }, 0)
    expect(onComplete(list, { agentId: 'b1', reason: 'answer' }, 9)[0]).toMatchObject({ status: 'done', endedAt: 9 })
    expect(onComplete(list, { agentId: 'b1', reason: 'max_turns' }, 9)[0]?.status).toBe('failed')
    expect(onComplete(list, { reason: 'answer' }, 9)).toEqual(list)
  })

  test("tool.call reads a task tool's arguments beside tool and agentId, and the created task's id", () => {
    let list = onSpawn([], spawn, { agentId: 'b1', model: 'm' }, 0)
    list = onToolCall(list, { tool: 'TaskCreate', agentId: 'b1', subject: 'write tests' }, { result: { task: { id: '7', subject: 'write tests' } } })
    list = onToolCall(list, { tool: 'TaskUpdate', agentId: 'b1', taskId: '7', status: 'in_progress' }, {})
    expect(list[0]?.tasks).toEqual([{ id: '7', content: 'write tests', status: 'in_progress' }])
    expect(onToolCall(list, { tool: 'TaskUpdate', agentId: 'b1', taskId: '7', status: 'completed' }, { isError: true })).toEqual(list)
    expect(onToolCall(list, { tool: 'TaskUpdate', agentId: 'b1', taskId: '7', status: 'completed' }, { result: { success: false } })).toEqual(list)
    expect(onToolCall(list, { tool: 'TodoWrite', todos: [] }, {})).toEqual(list)
  })

  test('session.end empties the list on clear and resume only; the rows tick while one runs', () => {
    const list = onSpawn([], spawn, { agentId: 'b1', model: 'm' }, 0)
    expect(onSessionEnd(list, 'clear')).toEqual([])
    expect(onSessionEnd(list, 'resume')).toEqual([])
    expect(onSessionEnd(list, 'logout')).toEqual(list)
    expect(ticking(list)).toBe(true)
    expect(ticking(onComplete(list, { agentId: 'b1', reason: 'answer' }, 1))).toBe(false)
    expect(ticking(onComplete(list, { agentId: 'b1', reason: 'answer' }, 1, 'waiting'))).toBe(true)
  })
})

describe('build-rows', () => {
  test('after the SVG, the build rows follow it grouped under lines holding the node IDs', () => {
    const all = text(flightRows(undefined, gitBuild(), 61_000))
    expect(all[0]).toBe('git')
    expect(all[1]).toMatch(/● build git/)
    expect(text(flightRows(CHART, gitBuild(), 61_000))).toEqual(text(flightLines(CHART, gitBuild(), 61_000)))
  })
})

describe('waiting', () => {
  const spawn = { prompt: PROMPT('git'), description: 'build git', subagentType: 'general-purpose' }
  const build = () => onSpawn([], spawn, { agentId: 'b1', model: 'claude-opus-5-5' }, 0)
  const rows = (list: AgentRun[], at: number) => between(text(flightLines(CHART, list, at)), '  git ', '  init ')

  test('a launcher prompt names its node on the second line', () => {
    const launcher = 'Read the file `.clm/prompts/git.md` with the Read tool.\nIts first step is `git switch impl/git`.'
    const list = started(['b1', 'build git', launcher])
    expect(rows(list, 5_000)[0]).toMatch(/●.*build git/)
    expect(text(flightLines(CHART, list, 5_000))).not.toContain('agents')
  })

  test('a build step quoted past the fifth line is no build', () => {
    const quoted = 'one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\n`git switch impl/git`'
    const all = text(flightLines(CHART, started(['x1', 'review', quoted]), 5_000))
    expect(between(all, '  git ', '  init ')).toEqual([])
    expect(all).toContain('agents')
  })

  test('an answer while the engine lists the agent waiting leaves a waiting row whose time grows', () => {
    const list = onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000, 'waiting')
    expect(list[0]).toMatchObject({ status: 'waiting' })
    expect(list[0]?.endedAt).toBeUndefined()
    const first = rows(list, 10_000)
    expect(first[0]).toMatch(/● build git · waiting$/)
    expect(first[2]).toContain('0:10')
    expect(rows(list, 20_000)[2]).toContain('0:20')
    expect(glyphs(flightLines(CHART, list, 0))).toEqual([{ text: '●', color: 'yellow' }])
  })

  test('an answer while the agent is listed any other way finishes the run', () => {
    for (const listed of ['completed', 'running', undefined]) {
      expect(onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000, listed)[0]).toMatchObject({ status: 'done', endedAt: 9_000 })
    }
    expect(onComplete(build(), { agentId: 'b1', reason: 'max_turns' }, 9_000, 'waiting')[0]?.status).toBe('failed')
  })

  test('a turn.start of a listed agent runs it again, drawn, keeping its tokens, dismissed or not', () => {
    let list = applyStep(build(), 'b1', 'claude-opus-5-5', undefined, usage(1_000_000))
    list = onComplete(list, { agentId: 'b1', reason: 'answer' }, 9_000, 'completed')
    expect(rows(list, 10_000)).toEqual([])
    const again = onTurnStart(list, { agentId: 'b1' })
    expect(again[0]).toMatchObject({ status: 'running', tokens: 1_000_000 })
    expect(again[0]?.endedAt).toBeUndefined()
    const row = rows(again, 12_000)
    expect(row[0]).toMatch(/● build git$/)
    expect(row[2]).toContain('1.0M tok')
    let failed = onComplete(build(), { agentId: 'b1', reason: 'max_turns' }, 9_000)
    failed = dismiss(failed, 'b1')
    const revived = onTurnStart(failed, { agentId: 'b1' })[0]
    expect(revived?.status).toBe('running')
    expect(revived?.dismissed).toBeUndefined()
    expect(rows(onTurnStart(failed, { agentId: 'b1' }), 12_000)[0]).toMatch(/● build git$/)
    expect(onTurnStart(list, {})).toEqual(list)
    expect(onTurnStart(list, { agentId: 'zz' })).toEqual(list)
  })

  test('a turn.step of a waiting or done run runs it again', () => {
    const waiting = onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000, 'waiting')
    expect(applyStep(waiting, 'b1', 'claude-opus-5-5', undefined, usage(10))[0]?.status).toBe('running')
    const done = onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000)
    const stepped = applyStep(done, 'b1', 'claude-opus-5-5', undefined, usage(10))[0]
    expect(stepped).toMatchObject({ status: 'running' })
    expect(stepped?.endedAt).toBeUndefined()
  })

  test('the 30-second reconciliation ends a waiting run the engine lists otherwise, and revives a lagging done one', () => {
    const waiting = onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000, 'waiting')
    const ended = reconcileWaiting(waiting, [{ id: 'b1', status: 'completed' }], 40_000)
    expect(ended[0]).toMatchObject({ status: 'done', endedAt: 40_000 })
    expect(rows(ended, 41_000)).toEqual([])
    expect(text(flightLines(CHART, ended, 41_000)).at(-1)).toMatch(/^≈\$/)
    expect(reconcileWaiting(waiting, [], 40_000)[0]?.status).toBe('done')
    expect(reconcileWaiting(waiting, [{ id: 'b1', status: 'waiting' }], 40_000)).toEqual(waiting)
    expect(reconcileWaiting(waiting, [{ id: 'b1', status: 'running' }], 40_000)).toEqual(waiting)

    const lagging = onComplete(build(), { agentId: 'b1', reason: 'answer' }, 9_000, 'running')
    expect(lagging[0]?.status).toBe('done')
    const back = reconcileWaiting(lagging, [{ id: 'b1', status: 'waiting' }], 40_000)
    expect(back[0]?.status).toBe('waiting')
    expect(back[0]?.endedAt).toBeUndefined()
    expect(rows(back, 41_000)[0]).toMatch(/● build git · waiting$/)
    const failed = onComplete(build(), { agentId: 'b1', reason: 'max_turns' }, 9_000)
    expect(reconcileWaiting(failed, [{ id: 'b1', status: 'waiting' }], 40_000)).toEqual(failed)
  })
})

describe('component-heading', () => {
  for (const heading of ['components', 'nodes']) {
    // req:plugin.panel/component-heading@900ca215
    test(`build rows sit under the lines after the ${heading} heading`, () => {
      const chart = ansiLines(`${heading}\n  git  P5\n  init  P5\nchanges\n`)
      const rows = between(text(flightLines(chart, gitBuild(), 61_000)), '  git ', '  init ')
      expect(rows.some(r => r.includes('build git'))).toBe(true)
    })
  }
})
