import { describe, expect, test } from 'vitest'

import { MACHINE_MS, machineArgs, machineLines, machineSampler } from '../src/machine.ts'
import type { Host, Run } from '../src/panel.ts'

const SHIM = '/plugins/clm/plugin/bin/clm'

/** A `$` with the `clm` shim installed, whose processes answer `answer`. */
function fake(answer: (argv: readonly string[]) => Promise<Run>) {
  const calls: (readonly string[])[] = []
  const $: Host = {
    plugin: { root: '/plugins/clm/plugin-panel' },
    process: {
      run(argv) {
        calls.push(argv)
        return answer(argv)
      },
    },
    fs: {
      async stat(path) {
        if (path === SHIM) return { kind: 'file' as const }
        if (path === '/work/repo') return { kind: 'dir' as const }
        throw new Error('ENOENT')
      },
      async list() {
        return []
      },
    },
  }
  return { $, calls }
}

const ROWS = '\x1b[33mcpu  ▄ 0.8× warn\x1b[0m\n\x1b[32mmem  ▂ 5.8G avail\x1b[0m\n\x1b[32mdisk ▄ 135G free\x1b[0m\n'
const cwd = async () => '/work/repo'

describe('machine-charts', () => {
  // req:plugin.panel/machine-charts@6e3ed26d
  test('runs clm machine at the pane width, every ten seconds', () => {
    expect(MACHINE_MS).toBe(10_000)
    expect(machineArgs(40)).toEqual(['machine', '--format', 'ansi', '--width', '40'])
    expect(machineArgs(Number.NaN)).toEqual(['machine', '--format', 'ansi', '--width', '100'])
  })

  // req:plugin.panel/machine-charts@6e3ed26d
  test('the rows are the run lines, coloured as their escapes say, and nothing before a run', async () => {
    const { $, calls } = fake(async () => ({ exitCode: 0, stdout: ROWS, stderr: '' }))
    const sampler = machineSampler($, cwd)
    expect(machineLines(sampler.state())).toEqual([])
    expect(await sampler.sample(40)).toBe(true)
    const lines = machineLines(sampler.state())
    expect(lines).toHaveLength(3)
    expect(lines[0]).toEqual([{ text: 'cpu  ▄ 0.8× warn', color: 'yellow' }])
    expect(calls).toEqual([[SHIM, 'machine', '--format', 'ansi', '--width', '40']])
  })

  // req:plugin.panel/machine-charts@6e3ed26d
  test('a sample reruns no clm view', async () => {
    const { $, calls } = fake(async () => ({ exitCode: 0, stdout: ROWS, stderr: '' }))
    const sampler = machineSampler($, cwd)
    await sampler.sample(80)
    await sampler.sample(80)
    expect(calls.every(c => c[1] === 'machine')).toBe(true)
    expect(calls).toHaveLength(2)
  })

  // req:plugin.panel/machine-charts@6e3ed26d
  test('starts no run while the previous one is still going', async () => {
    let finish: (r: Run) => void = () => undefined
    let slow = true
    let began: () => void = () => undefined
    const started = new Promise<void>(res => (began = res))
    const { $, calls } = fake(() => {
      began()
      return slow
        ? new Promise<Run>(res => (finish = res))
        : Promise.resolve({ exitCode: 0, stdout: ROWS, stderr: '' })
    })
    const sampler = machineSampler($, cwd)
    const first = sampler.sample(80)
    await started
    expect(await sampler.sample(80)).toBe(false)
    expect(await sampler.sample(80)).toBe(false)
    expect(calls).toHaveLength(1)
    finish({ exitCode: 0, stdout: ROWS, stderr: '' })
    await first
    slow = false
    await sampler.sample(80)
    expect(calls).toHaveLength(2)
  })

  // req:plugin.panel/machine-charts@6e3ed26d
  test('a failed run draws one dimmed line, in place of the rows or under the last good ones', async () => {
    let answer: Run = { exitCode: 1, stdout: '', stderr: 'boom\nmore\n' }
    const { $ } = fake(async () => answer)
    const sampler = machineSampler($, cwd)
    await sampler.sample(80)
    expect(machineLines(sampler.state())).toEqual([[{ text: 'machine: boom', dim: true }]])
    answer = { exitCode: 0, stdout: ROWS, stderr: '' }
    await sampler.sample(80)
    expect(machineLines(sampler.state())).toHaveLength(3)
    answer = { exitCode: 1, stdout: '', stderr: 'late' }
    await sampler.sample(80)
    const lines = machineLines(sampler.state())
    expect(lines).toHaveLength(4)
    expect(lines[3]).toEqual([{ text: 'machine: late', dim: true }])
  })

  // req:plugin.panel/machine-charts@6e3ed26d
  test('a run the engine aborted changes nothing', async () => {
    const { $ } = fake(async () => {
      throw new Error('process aborted')
    })
    const sampler = machineSampler($, cwd)
    expect(await sampler.sample(80)).toBe(false)
    expect(machineLines(sampler.state())).toEqual([])
  })
})
