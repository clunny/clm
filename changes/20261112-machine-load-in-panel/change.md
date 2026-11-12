---
status: applied
resolves: [panel-architecture-tab]
---

## Intent

The CLM pane drops its Capabilities tab and draws, above every tab, three small charts of the machine's load over the last 30 minutes: CPU, available memory and free disk, sampled every 10 seconds. A new command, `clm machine`, takes the readings, judges them (the disk against what this clone's builds have been measured to use), keeps the history and draws the rows the pane shows.

The Architecture-tab idea (`panel-architecture-tab`) is dropped: the pane is for watching the work and the machine while builds run, and the capability and architecture review stays with `clm capabilities`, `clm architecture` and the web app.

## Problem

Parallel builds load the machine heavily, and on 2026-11-07 six of them filled the disk partway through land checks (`builds-run-out-of-disk-mid-check`). Nothing beside the chat shows the load, so a person watching builds sees memory or disk running out only when something fails. The pane's Capabilities tab, meanwhile, is review material nobody reads while watching builds.

## Approaches

Where the readings come from:

1. **`clm machine` reads, judges, keeps the history and draws the rows; the pane shows its lines** (chosen, after the architecture audit found the pane drawing a fact `clm` can see, against `views-rendered-by-clm`). Only `clm` knows the clone's build places and `max_builds`, which the disk judgement needs; the pane's runtime has no Node.js; and the same measure can later stop a build from starting (`machine-load-measured-by-clm`).
2. **The pane runs `cat /proc/...`, `sysctl` and `df` itself.** Per-platform parsing in TypeScript, and it can't see the builds' disk use. Rejected.

How the disk a build needs is known:

1. **Measured from the builds' worktrees** (chosen, the human's call): at most every 10 minutes, the largest kept in a clone-local record.
2. **A size the human sets.** Wrong for every toolchain but the one it was set for, and stale as the project grows. Rejected.
3. **Measured only when a build lands.** Learns slowly and misses failed builds. Rejected.
4. **Inferred from drops in free space as builds start.** Noisy whenever anything else writes. Rejected.

How it is shown: a chart of the last 30 minutes sampled every 10 seconds, not a point gauge (the human's call), since the trend matters more than one reading.

## Outcome

- `plugin.panel` loses the Capabilities tab (`capabilities-tab` removed, `src/capabilities.ts` and its atom deleted) and gains `machine-charts`: it runs `clm machine --format ansi --width <columns>` every 10 seconds and shows its three rows above every tab.
- New component `machine` (crate `clm-machine`): `clm machine [--format text|json]` with CPU (5-minute load over the cores: `warn` from 1.25, `critical` from 2), memory (available, not free: `warn` below 20%, `critical` below 10%) and disk (against the builds' measured peak: `critical` when the running builds can't grow to it, `warn` when a build in every free place won't fit; 10% and 5% before any measurement), each `ok`, `warn`, `critical` or `unknown`. It keeps a 30-minute history of samples in the clone and draws it as `cpu`, `mem` and `disk` rows of block charts with the latest figure and, for a state other than `ok`, its word (`machine-history`).
- Root `clm` dispatches the command (`machine-command`), and its Design names the dispatch and the component.
- `CAPABILITIES.md` gains `watch-machine-load`.

## Constraints

- The pane decides and draws nothing of its own; the states and rows are `clm machine`'s (`views-rendered-by-clm`, `deterministic-enforcement`).
- Colour is never the only cue: every state but `ok` is written as a word (`colour-meanings`).
- Reuse a maintained crate for the readings (`reuse-first`), provided it cross-builds for the release targets.
- Under WSL the disk reading is the virtual disk's, which can exceed the host's free space.
