---
status: applied
resolves: [the-mutation-pass-judges-a-requirement-by-the-first-mutants-of]
---

## Intent

Plan work under the freeze (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`): the mutation pass is phase 3's evidence, and it files sound tests as unproven when the further mutants it tries are of code every test of the package runs. `ci/mutate-script` now takes a block's further mutants, the up to `limit` it tries beyond the diff's, by how much of their lines' running comes from the requirement's own tests, highest first, rather than in `cargo mutants --list` order, so the window holds the mutants of the code the requirement's tests are about (`backlog/the-mutation-pass-judges-a-requirement-by-the-first-mutants-of`). No change or code fix since 2026-11-12 touches the order of the further mutants; `ea7fe918` mended `machine`'s tests and left the order as it was.

## Problem

- `clm verify` puts every code file of a requirement's component in the plan with `limit 20` (`verify/mutation-pass`). With no diff since the base, `mutate.sh` tries the first 20 mutants the citing tests run, in `cargo mutants --list` order, and calls the requirement killed when any is caught.
- For `machine` those 20 are all in `crates/clm-machine/src/lib.rs`'s `State::word`, `State::from_word` and `judge`'s CPU and memory rules (`machine/machine-report`'s), which `read` runs for every test that calls it. So `machine/build-measure` and `machine/disk-room` were filed unproven by mutants of code neither requirement states, though a run over the whole crate showed their tests catch every mutant of their own code that isn't equivalent. The next pass would file them again: nothing a build of `machine` can add to those tests changes that, short of asserting CPU and memory states in tests that cite `build-measure`, which would make the verdict pass without proving the requirement.
- `ci/mutate-script` states the order (`in cargo mutants --list order`), so the fix changes a requirement (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: order the further mutants by their lines' share.** A line's share is its count in the block's own coverage run, the one that already finds the lines the citing tests run, over its count in a run of every test of the block's test packages. Code only the requirement's tests run has share 1 and comes first; code every test runs, such as `judge`'s CPU rule, has the citing tests' small fraction and comes last. In `clm-machine`, `judge_disk`'s room line runs only in tests whose record has a peak, so it ranks above the CPU and memory rules every `read` runs. It reuses the coverage tool and run the script already makes, plus one run of each package's tests, made only when a block has more tryable further mutants than its limit, and kept for the rest of the pass, so a later block naming the package reuses it. Ties keep the tool's order, so a crate whose lines all share alike is judged as now.
- **Rejected: order by how many of the citing tests run a line** (the item's second suggestion). Every test citing `build-measure` calls `read`, which runs the CPU and memory rules, so those would still come first.
- **Rejected: count, for each line, how many of the package's tests run it, fewest first.** It is the sharpest measure, but `cargo llvm-cov nextest` merges every test's profile, so it needs one run per test, or reading each test process's profile apart and telling a test's own process from the processes it starts; the share gets the same order for `machine` from two merged runs.
- **Rejected: lines only the citing tests run, then the rest in file order** (the item's first suggestion, read strictly). `disk-room`'s room line also runs in `build-measure`'s tests, so it would fall to the rest and the CPU rules would come first again; the share keeps this as its top tier and orders the rest.
- **Rejected: use the whole repository's `lcov.info` that `coverage.sh` wrote at the end of the last pass as the denominator.** It costs nothing, but it is of an earlier commit, whose line numbers needn't match, and absent on a first pass. Running `coverage.sh` before the blocks instead would make its failure, or a stalled npm install of its vitest half, sink every verdict of the pass, and spend the pass's `mutate_minutes` before the first verdict.
- **Rejected: the plan names fewer files** (the requirement's own code). `verify` can't tell which code of a component a requirement states (`verify/mutation-pass` keeps the component's files for this reason), and the diff already puts the code a land changed first.

## Outcome

- `ci/mutate-script` (modified): the further mutants are taken by share, highest first, and in `cargo mutants --list` order among equal shares; the run of every test of a package is made only when more further mutants may be tried than the limit, with `--ignore-run-fail` and no filterset, its report kept for the rest of the pass; it failing or writing no report fails the script as the block's own coverage run does. Four scenarios: *the further mutants of the requirement's own code first*, *no more further mutants than the limit*, *one run of a package's tests in a pass*, *the run of a package's tests fails*.
- `verify`'s plan and `limit 20` are unchanged.

## Constraints and edge cases

- The script is this repository's own; no file, key or format an adopter holds changes, and `clm land` gains no check (`mutation-after-land`).
- A pass gets longer by one instrumented run of each test package whose blocks have more than 20 tryable further mutants; the build is the one the block's own coverage run already made in its job's target directory. For `crates/clm`, whose tests are the bulk of the suite, that run is near the cost of the Rust half of `coverage.sh`, once a pass.
- `--ignore-run-fail` keeps a test of the package that fails in the run of every test from failing the pass: it changes only the order, and the land check has already passed the commit.
- A line the block's run counts more often than the package's runs (a test whose loop count varies) has share 1, not more.
- Counts include the processes a test starts (the `clm` binary the root's tests run), in both runs alike.
- StrykerJS's further mutants are chosen by StrykerJS as now; nothing has shown the same problem there (`yagni`).
- Two blocks running at once may both make the same package's run before either keeps it: each writes its report to a path of its own job and only then keeps it, so neither overwrites the other's while it reads it; a block that starts later reuses the kept one.
- From the builder check, for the build: whether further mutants may be tried is decided after the diff's run, so a diff mutant caught makes no package run; the kept report lives in the pass's temporary directory, not the block's, written to a temporary name and moved into place; shares are ordered by a stable sort to a file and then cut, as the further list already is, so a long list can't end the script with `SIGPIPE`; the fake `cargo` in `crates/clm/tests/ci_suite/ci_check.rs` tells the package run by its missing `-E` and gets per-line counts and an exit status for it alone; `cargo-llvm-cov` 0.9.1 takes `--ignore-run-fail` for `nextest`, writing a report and exiting 0 when a test fails and non-zero when the build does.
- The choice and what it rejected are recorded in `docs/decisions/mutation-after-land.md`.

## What the requirements no longer state

Nothing: `in cargo mutants --list order` becomes the order among equal shares.

## Open questions

None.
