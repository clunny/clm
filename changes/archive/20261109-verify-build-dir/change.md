---
status: done
resolves: [the-clone-s-shared-land-and-verify-build-caches-grow-without]
---

## Intent

`clm verify` hands `[verify] check` a build place of its own, `<home>/.clm/verify-build`, as `CLM_LAND_DIR`, the way `clm land` hands its check `.clm/land`. It clears that place after a run that leaves it over `[land] target_cap_gb`, with the cap code `clm land` already uses. A project's check script that points its toolchain at `CLM_LAND_DIR`, such as this repository's `ci-check.sh` or the starter `scripts/clm-check.sh`, then builds verify runs with incremental compilation off and in a directory that can't grow without limit.

## Problem

The backlog item reports two clone-wide build caches growing until the disk filled: the land's `.clm/land` and the build output inside `clm verify`'s checkout `.clm/verify-tree`. The land's half has since been fixed: `land/land-target-cap` clears `.clm/land` above `[land] target_cap_gb`, and `land/land-dir-env` with `ci/land-dir` turns incremental compilation off there.

Verify's half has not. `clm verify` runs `[verify] check` through `land/shared-check-runner` with no `CLM_LAND_DIR`, so a cargo check builds in `.clm/verify-tree/target` with the dev profile's incremental compilation on. The verify tree keeps git-ignored files from run to run, and nothing ever measures or clears them. On this clone that directory held 27 GB of debug output: 15 GB of incremental data, which helps only small edit-and-rebuild cycles and never a tree that jumps between whole commits, and 13 GB of compiled units for versions no run builds again, which Cargo never prunes.

The debug build itself is needed: it is the test build the check runs.

## Approaches

- **Its own place plus land's cap (chosen).** Verify sets `CLM_LAND_DIR` to `<home>/.clm/verify-build` for every run of `[verify] check`, and after each run, still holding the verify tree's lock, applies the cap `clm land` applies to `.clm/land`, taken from `[land] target_cap_gb`. `clm-land` offers that cap as a library call next to the shared check runner, so the two places are measured and cleared by one piece of code. Projects need no change: a check script that already handles `CLM_LAND_DIR` for lands handles it for verify too, and one that doesn't builds where it does today. The cap trades one cold verify run for a bounded directory, the same trade the land makes.
- **Its own place only.** Turns incremental off, which removes the larger part, but compiled units for old commits still pile up without limit, so the item would stay open.
- **A separate `[verify] target_cap_gb` setting.** Lets verify's cap differ from land's, at the cost of a settings key, its guide section and its config work. Nothing so far calls for a different value; the key can be added later if one does.
- **Share `.clm/land` with verify.** Both take the clone's check lock, so they wouldn't overlap, but a land and a verify run build different commits and would keep rebuilding each other's output. Rejected.
- **Use a new variable such as `CLM_BUILD_DIR`.** A clearer name, but every check script that handles `CLM_LAND_DIR` today would need an edit and a migration to benefit. The variable names the place the caller hands a check for its build output, so reusing it costs only a broader glossary entry.

## Outcome

After a verify run, `.clm/verify-build` holds the check's build output and is never above the cap once the run ends; `.clm/verify-tree` no longer accumulates a build directory for checks that honour `CLM_LAND_DIR`. A `target/` an earlier `clm` left inside `.clm/verify-tree` stays until removed by hand, since files git ignores stay in the verify tree; removing it is safe.

## Constraints

- The cap runs while verify still holds `verify.lock`, so no other verify run is building in the place it clears.
- A failure to measure or clear only warns, as the land's does: it never changes a run's record or exit status.
- `clm land` and `clm land --check-only` are unchanged.
