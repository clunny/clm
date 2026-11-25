---
status: applied
resolves: [one-block-s-strykerjs-failure-leaves-every-other-component-s]
---

## Intent

A block whose mutation tool fails no longer ends the mutation pass. Once the fixture has shown the tool works, a failure inside one block is that block's: `.github/scripts/mutate.sh` writes no verdict for it, goes on with the plan's other blocks, runs `coverage.sh` after them as usual, and still exits non-zero. A block that fails at every pass then costs only its own verdict. Today it keeps every requirement behind it unjudged for good, since `clm verify` puts what a pass left first in the next plan.

The item's second half, one failure filed as seven `unjudged-<component>` items on the components it starved, is designed by `20261124-an-unjudged-item-doesn`, which is in flight on its own spec branch. That change has `mutate.sh` write `<id> failed <why>`, and has `verify/files-unproven` count a requirement against a pass only when that requirement's own block failed. So this change designs only the first half, in `ci`, and adds no delta for `verify`.

## Problem

`backlog/one-block-s-strykerjs-failure-leaves-every-other-component-s`: in the passes after `bf0d9028` and `88470edf`, StrykerJS ran no test for five `web` blocks ("Vitest failed to find test files related to mutated files ... No tests were executed"). `ci/mutate-script` treats that as a tool failure and requires the script to "start no further block ... run no `coverage.sh`, and then exit non-zero". The pass recorded `0 killed, 0 unproven, 0 not judged, 29 left`. `verify/mutation-pass` puts left requirements first in the next plan, so the same five blocks would come first again and stop that pass too. While those blocks keep failing, no other requirement due is ever judged.

Stopping at the first failure makes sense for a tool that doesn't work at all. That case is already caught before any block starts: a tool that can't be installed, a fixture judged wrongly, or a missing `cargo-nextest` ends the script before its first block. The failures that `ci/mutate-script` lists inside a block belong to that block's own files and tests: `cargo mutants` failing in the unmutated tree, a filterset that doesn't parse, StrykerJS running no test, or the run that finds a block's lines failing. No requirement says that one block's failure must not cost the others their verdicts, so this goes into the spec (`code-fix-needs-a-written-rule`).

The cause of these particular five failures, StrykerJS's vitest runner relating tests to the mutated files instead of running the block's `test` lines alone as `ci/mutate-script` already requires, is a code-only fix in `mutate.sh`. `backlog/unjudged-ci-88470edf` holds it, and its own draft (`20261124-unjudged-ci-88470edf`) is fixing it. This change makes the next block-specific failure cost one block instead of the whole pass.

## Approaches

- **Taken: a block's failure ends only that block.** The script goes on starting the plan's blocks, writes each verdict as it is judged, runs `coverage.sh` after them, and exits non-zero. A failed block writes no verdict, so `verify/mutation-pass` leaves it and it stays due, as today. The exit status still tells `clm verify` that the command failed, so `mutation: <command> failed with status <n>` is still printed. Nothing in `verify` changes.
- **Not taken: have `verify/mutation-pass` put a requirement left by a failed block last in the next plan instead of first.** That would need `verify` to know which block failed, which only `20261124-an-unjudged-item-doesn` provides. The failing block would still end the pass, so whatever came after it would wait one pass longer. And a requirement starved for a reason other than its own block would lose its place.
- **Not taken: tell a "tool broken" failure from a "this block" failure by its message, and stop only on the first kind.** No message reliably tells them apart: StrykerJS's "no tests were executed" was block-specific here, but the same text could come from a broken vitest. The fixture is what shows whether the tool works, and it already runs before every block.
- **Not taken: accept it.** Five blocks that fail at every pass would keep every other requirement due from its verdict for as long as they fail. The mutation pass is phase 3's evidence under `backlog/a-passing-check-proves-less-than-the-spec-claims`, so a pass that judges nothing undoes the plan's measure.

## Outcome

- `ci/mutate-script` (modified): when a block fails once blocks have started, for one of the listed reasons or any other (a tree copy it can't make, a mutant it can't write), the script leaves nothing a later block reads, such as a part-made copy of the tree for its job number, writes its `<id> failed <why>` line (from `20261124-an-unjudged-item-doesn`) and no verdict for it, goes on starting the plan's other blocks, runs `coverage.sh` after them, and exits non-zero. The scenario *a failing block lets the others finish* now expects the third block's `killed` line, the first block's `failed` line and `coverage.sh` run. A new scenario, *a StrykerJS block that runs no test lets the next start*, covers the failure seen after `88470edf`, with a `failed` line naming StrykerJS for the first block.
- `ci`'s Design, paragraph **Mutation**: adds why a block's failure is the block's alone, and that the script records its `failed` line and no verdict.
- `ci`'s Contract, the `mutate.sh` bullet: names the `<id> failed <why>` line and that the script goes on past a failed block.
- `ci/mutate-script` also states, from the fold's builder check, the why of a block that ended without printing a `mutate.sh: ` line (`exited with status <n>`), and that a block `SIGTERM` or `SIGINT` ended writes no `failed` line.
- `docs/decisions/mutation-after-land.md` gains a dated section for this change.
- No capability test sees a difference. None runs `mutate.sh`, and `clm verify`'s outcome line, stdout and exit status are unchanged.

## Constraints and edge cases

- A tool that breaks part-way through a pass, after its fixture passed, now fails every block it is given instead of only the first. Each such failure is quick (a baseline test run or a StrykerJS dry run), and `[verify] mutate_minutes` bounds the whole pass. It records no verdict for those blocks, as today, and a `failed` line for each.
- A run of every test of a package that fails is still kept for no later block, so each block of that package runs it again and fails again. This costs time and records nothing wrong, and the time limit bounds it.
- `coverage.sh` now runs after a pass in which a block failed. A failure of `coverage.sh` still makes the script exit non-zero, as it does today.
- `20261124-an-unjudged-item-doesn` modifies the same requirement, and changes the same scenario, *a failing block lets the others finish*: there the first block gets a `failed` line instead of no line. The fold of both onto one spec branch merged that change's edit into this delta, which applies after it. The two outcomes compose: the first block gets its `failed` line, and the third block is judged. Under that change's filing rule, as the fold settled it, a pass that goes on past a failed block and ends with its status leaves no other requirement on the failure's account, so it files only on the failed block's component; a pass that reaches its limit or a signal counts what it left on each requirement's own component, as the scenario *a limit reached with a failed block* shows.
- Build order: `ci/mutate-script` keeps its `#### Needs` on `verify/mutation-pass`, which names the line format the script writes and is behind under `20261124-an-unjudged-item-doesn`, so it is built after `ledger/mutated-record` and `verify/mutation-pass`. Today's verify ignores a line it can't parse, so a `failed` line written early would only leave the requirement, as no line does.
- Per-job state now outlives a failed block, so the build must not leave a part-made tree copy that a later block on the same job number would reuse (the builder check found `judge_test_files` making `copy-<job>` only when it is missing). The requirement states this.
- The existing test that asserts the old behaviour (no line for the third block, no `coverage.sh`) is rewritten by the build under the scenario's kept name. The fake `npx` gains a per-block switch for the new StrykerJS scenario.

## What the requirements no longer state

- `ci/mutate-script`: "When a block fails so, it SHALL start no further block, let the blocks already running finish and write their lines, run no `coverage.sh`, and then exit non-zero." The script now goes on starting the plan's other blocks and runs `coverage.sh`, and still exits non-zero.
- `ci/mutate-script`: "So a tool that doesn't work records no verdict for the block it failed in, and that block and every block not started stay due." Only the failed block stays due now, since every other block is started.
- `ci/mutate-script`, scenario *a failing block lets the others finish*: the outcome "no line for ... the third, no call named the third block's files, and `coverage.sh` didn't run" becomes the third block's `killed` line and `coverage.sh` run.

## Open questions

- **Is this the plan's work, or does it wait for the freeze to end?** The draft judged it plan work, as `20261124-the-mutation-pass-s` and `20261123-the-mutation-pass-can` were: the mutation pass is phase 3's evidence, and a pass that one block's failure stops judges nothing. If the human would rather hold it, add `ci/mutate-script` to the plan item's `blocks`.
