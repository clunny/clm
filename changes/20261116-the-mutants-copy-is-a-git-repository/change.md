---
status: applied
resolves: [verify-s-mutation-baseline-fails-a-clm-health-test-needs-a-git]
---

## Intent

Plan work under the freeze (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`): phase 4 broke the mutation pass that phase 3 built, and this mends it. `cargo mutants` tests a copy of the tree that holds no `.git`. Since `commands-in-crate` moved each component's tests into its own crate, the tool's unmutated baseline runs tests that ask git about this repository, they fail in the copy, and the pass judges nothing: three verifies in a row recorded nothing. `ci/mutate-script` now has the script make the copy a git repository.

## Problem

`clm verify --due` on 2026-11-15 failed three times at the mutation step with every test passing in the checkout. The failing test was `clm-health`'s `scenario_a_clean_repository`, which runs `clm health`'s call, which runs `git ls-files`. Before phase 4 it sat in the root package, where the tool's baseline for a mutant of `clm-health` never ran it. Other moved tests read the repository the same way (`clm-check`'s `this_repository_passes_its_rules`, `clm-config`'s `this_repository.rs`), so the next component judged would fail too.

## Approaches

- **Have the tool copy `.git`** (`--copy-vcs true`, which the pinned `cargo-mutants` 27.1.0 offers). Taken. Measured on 2026-11-16 in a worktree like verify's, at `8a9d04cc`: without it the baseline fails on that test and the tool exits 4; with it the baseline passes and five mutants are judged. The tree verify runs the step in is a git worktree, so there is a `.git` to copy.
- **Mark such tests so the pass leaves them out.** Not taken: they are the tests of the requirements the pass is there to judge.
- **Make each such test build its own repository.** Not taken: a test that this repository passes its own rules must read this repository.
- **Mutate in place** (`--in-place`). Not taken: it writes mutants into the verify tree, which a stopped run would leave changed.

## Outcome

`ci/mutate-script` states that `cargo mutants` copies the checkout's `.git` into the tree it tests, with a scenario. The fixture the script judges before the plan becomes a git repository whose tests pass only inside one, so every pass proves with the real tool that the copy is a repository: a tool or a version that stops copying `.git` fails the fixture and the pass, loudly, where it would otherwise judge nothing again (`mutation-after-land`). `ci`'s build adds the argument to `.github/scripts/mutate.sh`. The report named `check.health`; the fix lies in `ci`.

## What it removes

Nothing.

## Constraints

- No file, key or format an adopter holds changes: the script is this repository's own.
- `clm land` gains no check; the pass runs after the land.

## Open questions

- In the measurement, the two mutants that built were both missed by `scenario_a_clean_repository`: the function `finding` returned an empty string and the test passed. Five mutants judge nothing, but the first real pass over `check.health` may file an `unproven` item.
- From the architecture audit, left: nothing but today's tests keeps a test in the copy from writing to this repository through git, which `tests-run-concurrently` would not allow. No program can see that, and a copy with a git directory of its own would no longer be this repository, which is what these tests read.
- The copy's `.git` is the worktree's link to the clone's git directory, so a test in the copy reads the verify tree's index. A test that wrote to this repository through git would write there; none does today.
