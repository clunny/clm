---
status: applied
resolves: [the-verify-tree-s-build-caches-grow-without-bound-and-take-the]
---

## Intent

`clm verify` leaves its tree, `.clm/verify-tree`, holding only what git tracks once it is done with it, so what a project's check or mutation command builds inside the tree can't pile up between runs and take the build loop's room (`backlog/the-verify-tree-s-build-caches-grow-without-bound-and-take-the`). The place a project keeps from run to run stays `CLM_LAND_DIR`, `.clm/verify-build`, which the cap already bounds (`verify-build-cap`). No change or code fix since 2026-11-12 touches the verify tree's ignored files.

## Problem

- **The tree keeps what git ignores.** Each checkout into the verify tree runs `git clean -fd`, which removes untracked files but keeps ignored ones, so a build directory would be reused (the doc comment of `Tree::at` in `crates/clm-verify/src/tree.rs`). The Design moved the check's own build output to `.clm/verify-build` and capped it, but nothing bounds what any command still writes in the tree.
- **This repository's pass builds there.** `mutate.sh` sets each job's `CARGO_TARGET_DIR` to `target/mutate-job-<n>` under the tree (`CARGO_TARGET_DIR` unset, as verify sets none), and `coverage.sh`'s `cargo llvm-cov` builds `target/llvm-cov-target`. After a day of verifies those held 72 GB (five jobs of 10-12 GB and 15 GB of coverage build). The build loop sets aside about 21 GB per running build before it counts free room (`machine/disk-room`), so the queue showed builds waiting for disk until the human found the directories and deleted them.
- No requirement states what the verify tree may keep, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: verify cleans its tree when it is done with it.** After the mutation pass (or the record, when none runs) and before filing, still under the verify tree's lock, `git clean -ffdx` at the tree's top. It bounds the tree for every project, whatever its scripts write there, with one call verify already makes the shape of (`reuse-first`); the bound for build output stays the one place the cap governs. Checkouts within one `clm verify` keep ignored files as now, so the `node_modules/` the check installs still serves the pass's StrykerJS.
- **Rejected: fix this repository's `mutate.sh` alone** (build the jobs and coverage under `$CLM_LAND_DIR`, or remove the job targets at exit). It fixes this project and leaves the next script that builds in the tree to fill the disk again. Building under `$CLM_LAND_DIR` is worse still: the jobs' 60 GB would push the place over its 15 GB cap after every check, clearing the check's warm build and `lcov.info` (`config/health-lcov-here`) each time.
- **Rejected: the loop's room check names the verify tree's caches as reclaimable.** It tells the human where the room went and leaves them to delete it by hand, the work this change exists to take off them.
- **Rejected: one target shared by every mutation job.** Cargo locks a target directory per build, so the jobs would wait on one another and the pass's parallelism would be lost.
- **Rejected: clean ignored files at every checkout.** The pass checks the tree out afresh after the check, so it would remove the `node_modules/` the check installed and the pass's StrykerJS blocks run from.

## Outcome

- `verify/verify-tree-cleared` (added): every `clm verify` that checked a commit out into the verify tree removes, before it returns, after the mutation pass and before filing, under the tree's lock, every file git doesn't track there, ignored ones included; only when git finds the tree's own top there, otherwise saying `clm verify: cannot clean .clm/verify-tree: <why>` on stderr; nothing about it changes the record, the outcome line, stdout or the exit status.
- `verify` Design: a paragraph *A clean tree between runs* gives the rule's reasons and costs; the *Build place* sentence no longer says the tree keeps what git ignores from run to run.
- `docs/guide/ci-and-deployment.md`: one sentence telling a project that the verify tree keeps nothing between calls.

## Constraints and edge cases

- **The cost.** Each pass now builds each job's target cold, where it was warm from the last pass; a pass's peak, its jobs' targets while it runs, is room the work uses and stays. Accepted: verifies are rare (`queue-drained`, every 20 lands here), and the pass already runs up to an hour.
- **A tree that isn't whole.** Git run in a tree whose `.git` is gone finds the main worktree around it, and `git clean -ffdx` there would remove the clone's `.clm/`. The clean runs only when git finds its top at the tree itself, the test the checkout already makes before reusing it.
- **A killed call.** A `clm verify` killed with `SIGKILL` cleans nothing; the next call that checks the tree out cleans what it left, since the clean takes the whole tree. A stopped pass (`SIGTERM`) ends through verify's own handler, so its call still cleans.

## Open questions

None.
