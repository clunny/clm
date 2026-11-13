---
status: proposed
---

## Intent

The machine rows the CLM pane shows keep, in every chart cell, the colour of the state its sample had when it was taken, so the last half hour reads as what happened then: a burst of amber stays amber after the load falls back. Samples in `<git common dir>/clm/machine-samples` therefore record each reading's state beside its share. The row's label, figure and state word stay those of the current reading.

CPU also flags sooner, as the human chose: `warn` (amber) when the **1-minute** load over the logical cores is at least 1.25, and `critical` (rose) when the **5-minute** load over the cores is at least 1.25, `critical` taking precedence. Until now it was judged on the 5-minute ratio alone, `warn` from 1.25 and `critical` from 2.

## Problem

Each sample holds only the CPU, memory and disk shares, and each whole row is painted in the latest reading's state. When the current reading turns `ok`, every earlier cell turns green too, so a spell of `warn` or `critical` vanishes from the chart the moment it ends, which is exactly the history the chart is there to keep (`machine-load-measured-by-clm`: "a history, not a gauge"). The state can't be worked out again when drawing: the disk's state depends on the measured `peak` and the free places at that moment, and the CPU's on the 5-minute load, neither of which the share holds.

The CPU thresholds flagged too late for the human: a parallel cargo build fills every core, so a ratio near 1 is normal while building, but sustained queueing up to 2× the cores showed only amber, and a minute of heavy queueing showed nothing.

## Approaches

Where a cell's colour comes from:

1. **Each sample records the state its reading was judged at, and each cell is painted in it** (chosen). Correct for all three readings, the disk included, and the state recorded is the one the pane showed at that moment.
2. **Judge each sample again from its share when drawing.** Can't be right for the disk, whose state depends on the `peak` and the free places of that moment, nor for CPU, whose `critical` rests on the 5-minute load the share doesn't hold. Rejected.
3. **Record the raw readings and judge them when drawing.** Needs the record and places as they were then too; more stored to reach the same answer as 1. Rejected.

How a sample written before samples held states is read:

1. **It is no sample: dropped like any malformed line when the file is next written** (chosen). Such lines are at most 30 minutes old, so the cost is one half-hour of chart, once, after the upgrade. The code needs no second parser that would outlive its use by forever (`no-dead-code`, `yagni`).
2. **Read with each state `unknown`, cells dimmed.** `colour-meanings` reads dimmed as "the system gave no reading", which would be false: the reading was taken, only its state wasn't kept. And the parser for the old form is dead 30 minutes after the upgrade. Rejected.
3. **Read with each state `ok`.** Paints green over moments that may have been amber or rose: the very fault this change fixes. Rejected.

CPU thresholds (the human's call, after being told a parallel cargo build fills every core): `warn` on the 1-minute ratio from 1.25, `critical` on the 5-minute ratio from 1.25. The 1-minute load flags queueing as it starts; the 5-minute one turns it rose once it has held for minutes, so a spike shows amber and sustained queueing rose. Rejected: keeping the 5-minute ratio alone with lower thresholds, which still lags a minute's queueing by minutes.

## Outcome

- `machine-report`: CPU is `critical` when `load5 / cores` is at least 1.25, else `warn` when `load1 / cores` is at least 1.25, else `ok`; `unknown` without a load average.
- `machine-history`: a sample holds each reading's share and state word; each chart cell is coloured by its sample's state, and the label, figure and state word by the current reading's; a line of another shape, the old four-field form included, counts as none and is dropped.
- Decisions `machine-cpu-flags-the-minute` and `machine-cells-keep-their-state`, each amending `machine-load-measured-by-clm`.

## Constraints

- The JSON and text forms keep their fields; only the CPU state they report moves with the new thresholds.
- The pane draws the rows as their escapes say, run by run (`plugin.panel/machine-charts`, whose `ansiLines` already reads several styled runs per line), so `plugin.panel` needs no change; each coloured run ends with a reset so a dimmed cell's style never carries into the next.
- The history file is clone-local state that lives 30 minutes, not a format projects keep, so no `upgrade` migration is owed (`upgrades-carry-migrations`).
