---
title: "The mutation pass rides verify's trigger and makes every verify slow"
kind: idea
components: [verify, config, orchestrator.loop]
---

`clm verify` runs two things with different purposes on one trigger (`[verify] triggers = ["queue-drained"]`, `every = { lands = 20 }`):

- **The full suite** (`[verify] check`, `ci-check.sh`): the safety net for lands that check only where the branch and `main` overlap. It answers "is `main` green?" in 3–5 minutes (156s and 275s on 2026-11-15) and is worth hearing about soon.
- **The mutation pass** (`[verify] mutate`, `mutate.sh`, with `coverage.sh`): it answers "do the tests prove what they cite?", the evidence the plan `a-passing-check-proves-less-than-the-spec-claims` runs on. It took 30–40 minutes per run on 2026-11-15, and nothing waits on its verdicts.

Because they share a trigger, the quick answer waits on the slow one and every verify costs about 40 minutes, the build loop's `run verify` with it. During that time the human sees development stall.

## The idea

Separate the layers by purpose, each with a time budget:

| Layer | Budget | Runs | When |
|---|---|---|---|
| Land | under 3 min | overlap-selected tests, `clm check` | every land (as now) |
| Verify | under 10 min | the full suite, no selection, and coverage | today's triggers |
| Mutation audit | up to 1 hour | the mutation pass on the requirements that are due | its own trigger or schedule: nightly, when idle, or every M lands |
| Release | none of its own | nothing new: a release needs a green verify and no mutation verdict due | before a release |

The human's condition (2026-11-15): the separate audit is preferred **if it can run concurrently while other components build**, so it never holds the loop or a land. Whether it can is the first thing to settle: the pass mutates files in its own tree and runs `cargo-mutants` and Stryker, which compete with builds for CPU and the cargo target directory, and the loop must not wait on its return before launching builds.

No full mutation run before a release: the pass already judges per requirement and leaves a requirement due when a tool fails (`verify/mutation-pass`), so a regular audit leaves little for a release to find, and "nothing due" is a cheap gate.

## Cheap wins either way

- **Timeouts dominate.** The slowest batches (5 and 10 minutes) were mostly mutants that hang until their limit, the ones that send loop or land code into an endless wait. A tighter `cargo-mutants` timeout (`--timeout-multiplier`) cuts that without losing verdicts.
- **Time-box the audit.** Stop at the budget and leave the rest due for the next run, so a run stays bounded however many lands it fell behind, and a large batch of changes spreads over several runs.
- **Record each stage's time.** `ci-check.sh` prints per-step seconds and `cargo-mutants` per-batch times, but the coverage pass prints no total and `clm verify` records no per-stage duration, so a budget can't yet be checked.
