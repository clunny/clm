---
title: "A mutation pass that stops starts over, runs one mutant at a time, and has no limit on its time"
kind: idea
components: [verify, ci]
priority: P2
---

## Intent

Plan work, by the human on 2026-11-16: it concerns the evidence phase 3 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md` built, and phase 4 exposed it. A mutation pass should keep what it has judged when it stops, use the machine it runs on, and take a bounded time. The human set it at `P2` on 2026-11-16.

## Problem

Measured on 2026-11-16, on a `clm verify --due` that the build loop started after phase 4's builds:

- The pass held 219 requirements with 1,560 tests, where a pass usually holds a handful. Phase 4 moved about 700 tests into their components' crates in one day, and a moved test counts as changed (`mutation-after-land`).
- It took about 46 seconds a requirement: 177 judged after 2 hours 16 minutes, with 42 and the coverage run still to go. At that point 169 were `killed`, 2 `survived` and 6 `not-judged`.
- **A pass records its verdicts only when it completes** (`verify/mutation-pass`). The passes before it had failed three times on a test that needed a git repository and once on a hang, each recording nothing, so the whole day's requirements stayed due and every retry began again from the first. Stopping this one by hand would have thrown away two hours the same way.
- **Nothing limits a pass.** The human waits on "main is verified" for as long as the pass has requirements. The plan settles a limit for evals ("each run has a spend limit, orders what is due within it"); the mutation pass has none.

- **The pass is serial at every level.** During that pass the machine, with 24 cores and 15 GB of memory, showed a load average of 0.5. `.github/scripts/mutate.sh` judges the plan's blocks one after another, and `cargo mutants` tests one mutant at a time. In one block of `clm-orchestrator` the baseline built in 6.8 seconds and its tests took 10.5; each mutant built in 0.7 seconds and its tests took 10.5 again, with the processor idle, so a block of 13 mutants and then more took about two minutes for one requirement.

The rest of phase 4 moves about 210 more tests (`ci`, the `plugin` family, `view`, `check.coverage`), so large passes will come again before the plan ends.

## Approaches

Not yet weighed against the code; a session that proposes this reads `clm context verify`, `crates/clm-verify/src/mutate.rs` and `.github/scripts/mutate.sh` first.

- **Record each verdict as it is reached**, so a pass that fails or is stopped leaves its verdicts and the next pass judges only what is left. To settle: where a partial result is kept (the ledger holds one `mutated` record a pass today), and what a tool failure part-way records.
- **A limit on a pass**, in time or in requirements, with what is due ordered and the rest carried to the next pass. To settle: the order (newest lands first, or changed code first as today), whether the limit is a setting, and how `clm verify` says what it left.
- **Several requirements at once** in this repository's own script (`ci/mutate-script`). Each block is its own `cargo mutants` call with its own copy of the tree and its own build directory, so blocks share nothing but the compiler cache; four to six at once would have taken that pass from about three hours to 30 to 45 minutes. To settle: how many, since memory and not cores is the limit here (each concurrent build of a crate's tests takes one to two gigabytes) and the number should follow the machine limits the build loop already reads (`clm machine`, `max_builds`); how the result file is written by several writers; and whether `cargo mutants --jobs`, which runs several mutants of one block at once, is added as well or only if it is still slow, since the two multiply each other's memory. The tests must pass beside each other, which `tests-run-concurrently` already asks; one that fails only here is a finding. This part is `ci`'s and needs nothing of `verify`.
- **Not judging a test that only moved.** Not recommended: most of these tests were written before the pass existed and had never been judged, so this pass is the first evidence that they notice anything.

## Open questions

- Whether the three parts are one change. Recording verdicts as they come would have saved every retry on 2026-11-15 and 2026-11-16; running blocks at once shortens every pass and is the script's alone; the limit matters least once the other two are in.
- Why a block's tests take 10.5 seconds with the processor idle: probably one test that waits on a timeout, which is worth finding whatever else is done.
- Whether a pass's verdicts so far should reach `main` before the pass ends, as `unproven` items do when it completes.
