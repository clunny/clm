---
status: done
resolves: [clm-verify-s-coverage-step-fails-because-clm-verify-build-doesn]
---

## Intent

`clm verify` creates its build place, the main worktree's `.clm/verify-build`, before every run of `[verify] check` and of `[verify] mutate` it hands that place as `CLM_LAND_DIR`, so a command can write there without making it first (`backlog/clm-verify-s-coverage-step-fails-because-clm-verify-build-doesn`). The item's other complaint, a run that exits 0 after `mutation: ... failed with status 1` with a last line counting 9 killed, is accepted as it stands: the pass never changes the outcome or exit status by design (`mutation-pass`, `mutation-after-land`), the failure has its own line before the count, and the 9 verdicts were each written before `coverage.sh` failed, so the count is true.

## Problem

`verify-build-dir` names `.clm/verify-build` in `CLM_LAND_DIR` but nothing creates it. `verify-build-cap` removes it whole after a check that leaves it over the cap, which a full Rust build of this repository does, and the mutation pass then runs with `CLM_LAND_DIR` naming a missing directory. `.github/scripts/mutate.sh` ends by copying `target/lcov.info` to `$CLM_LAND_DIR/lcov.info` (`ci/mutate-script`); `cp` fails with "No such file or directory", the script exits 1, and the tracefile `clm health` reads (`config/health-lcov-here`) is never written. A fresh clone's first run has the same hole for any check that writes there before its toolchain creates the directory.

## Approaches

1. **`clm verify` creates the place before each command it hands it to** (chosen). One `create_dir_all` in the crate that names the place, before the check and before the mutate command. Every project's scripts get a place that exists, whatever their toolchain; the cap stays as it is.
2. **`mutate.sh` runs `mkdir -p "$CLM_LAND_DIR"` before the copy.** Fixes this repository's script only; every other project's `[verify] mutate` or check that writes into `CLM_LAND_DIR` meets the same missing directory that verify itself made missing. A workaround in each consumer for a hole in the producer (`redesign-before-workaround`).
3. **The cap empties the directory instead of removing it.** Changes `land`'s shared cap (`land/shared-target-cap`) and its scenarios for both callers, and still leaves a fresh clone's first run without the place.

## Outcome

- `verify-build-dir` adds: before each check run (a bisect probe's included) and each mutate run, verify creates `.clm/verify-build` when missing; a failure to create it is said on stderr as `clm verify: cannot create .clm/verify-build: <error>` and changes nothing about the run's result.
- The Design's **Build place** paragraph says so, and why.
- After the next pass on this clone, `.clm/verify-build/lcov.info` exists and `clm health` reads coverage from it.

## Constraints

- Nothing about the place may lose a run's result (`verify-run`'s **Output**): creation failing is a note, not an error.
- `land`'s `.clm/land` is left as it is: no land's check is known to write there before its toolchain builds, and nothing removes it between a land's check and another command that gets it (`yagni`).

## Edge cases and workarounds

- A regular file where the place should be: creation fails, the line is printed and the command runs as before.
