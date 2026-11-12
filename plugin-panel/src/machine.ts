// The machine-load rows above every tab (`machine-charts`): `clm machine --format ansi` draws
// them, the pane shows its lines as `ansiLines` reads them. Only the latest output is kept, in
// memory, since the 30-minute history is `clm`'s.
import { type Host, type Span, ansiLines, runClm, widthOf } from './panel.ts'

/** How often an open pane samples the machine. */
export const MACHINE_MS = 10_000

/** `clm machine` for a pane `columns` wide. */
export const machineArgs = (columns: number): string[] => ['machine', '--format', 'ansi', '--width', String(widthOf(columns))]

/** The latest successful run's lines, and the first line of stderr of a run that failed since. */
export type MachineState = { rows?: Span[][]; failed?: string }

/** The lines drawn above a tab's content: the rows, then `machine: <why>` dimmed after a failed run. */
export function machineLines(state: MachineState): Span[][] {
  const lines = state.rows ?? []
  if (state.failed === undefined) return lines
  return [...lines, [{ text: `machine: ${state.failed}`, dim: true }]]
}

/**
 * The pane's sampler: `sample` runs `clm machine` unless the previous run is still going, and
 * answers whether it changed what is drawn. A run the engine aborted changes nothing.
 */
export function machineSampler($: Host, cwd: () => Promise<string>) {
  let state: MachineState = {}
  let running = false
  return {
    state: (): MachineState => state,
    async sample(columns: number): Promise<boolean> {
      if (running) return false
      running = true
      try {
        const run = await runClm($, machineArgs(columns), await cwd())
        state =
          run.exitCode === 0
            ? { rows: ansiLines(run.stdout) }
            : { ...state, failed: run.stderr.trim().split('\n', 1)[0] || `clm exited ${run.exitCode}` }
        return true
      } catch {
        return false
      } finally {
        running = false
      }
    },
  }
}
