---
component: config
---

## MODIFIED Requirements

### Requirement: verify-here <!-- base: b11aa28496558b25 -->
This repository's `clm.toml` SHALL set `[verify]` with `check = ".github/scripts/ci-check.sh"`, `mode = "local"`, `triggers = ["queue-drained"]`, `every = { lands = 20 }` and `mutate = ".github/scripts/mutate.sh"`, and no other key of the table, so the whole suite runs after lands that checked only where the branch and `main` overlap (`land/overlap-recheck`), on this machine, once the build queue drains and at least every 20 lands, and the mutation pass follows each run that passed (`verify/mutation-pass`, `ci/mutate-script`).

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get verify` runs against it
- THEN it prints `mode=local,check=.github/scripts/ci-check.sh,triggers=queue-drained,every.lands=20,mutate=.github/scripts/mutate.sh` and the file's path

#### Serves
- capability/land-safely
- capability/prove-by-tests

#### Needs
- ci/mutate-script

### Requirement: health-lcov-here <!-- base: 1b7840261d716532 -->
This repository's `clm.toml` SHALL set `[health]` with `lcov = ".clm/verify-build/lcov.info"`, the copy of the tracefile `.github/scripts/coverage.sh` writes that the mutation pass leaves in the place `clm verify` keeps from run to run (`ci/mutate-script`), and no other key of the table. The path is under the clone's main checkout, so `clm health` reads it there and reports `not found` in a linked worktree.

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get health` runs against it
- THEN it prints `reconsider=3,scenarios=6,exceptions=4,stale_days=30,lcov=.clm/verify-build/lcov.info` and the file's path

#### Serves
- capability/know-health

#### Needs
- ci/coverage-script
