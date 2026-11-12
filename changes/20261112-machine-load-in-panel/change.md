---
status: proposed
resolves: [panel-architecture-tab]
---

## Intent

The CLM pane drops its Capabilities tab and draws, above every tab, three small charts of the machine's load over the last 30 minutes: CPU, available memory and free disk, sampled every 10 seconds. A new command, `clm machine`, takes the readings and judges them, the disk against what this clone's builds have been measured to use.
