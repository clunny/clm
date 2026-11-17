---
title: "A mutation pass that stops starts over, and nothing limits how long one takes"
kind: idea
components: [verify]
---

## Intent

Plan work, by the human on 2026-11-16: it concerns the evidence phase 3 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md` built, and phase 4 exposed it. A mutation pass should keep what it has judged when it stops, and should take a bounded time.

## Problem

Measured on 2026-11-16, on a `clm verify --due` that the build loop started after phase 4's builds:

- The pass held 219 requirements with 1,560 tests, where a pass usually holds a handful. Phase 4 moved about 700 tests into their components' crates in one day, and a moved test counts as changed (`mutation-after-land`).
- It took about 46 seconds a requirement: 177 judged after 2 hours 16 minutes, with 42 and the coverage run still to go. At that point 169 were `killed`, 2 `survived` and 6 `not-judged`.
- **A pass records its verdicts only when it completes** (`verify/mutation-pass`). The passes before it had failed three times on a test that needed a git repository and once on a hang, each recording nothing, so the whole day's requirements stayed due and every retry began again from the first. Stopping this one by hand would have thrown away two hours the same way.
- **Nothing limits a pass.** The human waits on "main is verified" for as long as the pass has requirements. The plan settles a limit for evals ("each run has a spend limit, orders what is due within it"); the mutation pass has none.

The rest of phase 4 moves about 210 more tests (`ci`, the `plugin` family, `view`, `check.coverage`), so large passes will come again before the plan ends.

## Approaches

Not yet weighed against the code; a session that proposes this reads `clm context verify`, `crates/clm-verify/src/mutate.rs` and `.github/scripts/mutate.sh` first.

- **Record each verdict as it is reached**, so a pass that fails or is stopped leaves its verdicts and the next pass judges only what is left. To settle: where a partial result is kept (the ledger holds one `mutated` record a pass today), and what a tool failure part-way records.
- **A limit on a pass**, in time or in requirements, with what is due ordered and the rest carried to the next pass. To settle: the order (newest lands first, or changed code first as today), whether the limit is a setting, and how `clm verify` says what it left.
- **Not judging a test that only moved.** Not recommended: most of these tests were written before the pass existed and had never been judged, so this pass is the first evidence that they notice anything.

## Open questions

- Whether both parts are one change. The first alone would have saved every retry on 2026-11-15 and 2026-11-16; the second alone only shortens the wait.
- Whether a pass's verdicts so far should reach `main` before the pass ends, as `unproven` items do when it completes.
