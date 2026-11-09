---
status: applied
resolves:
  - land-still-sets-cargo-variables
---

## Intent

`clm land` stops setting cargo's variables for a project's check. The starter `scripts/clm-check.sh` that `clm init --lang rust` writes points cargo at `CLM_LAND_DIR` itself, an advised migration rewrites an unedited older script to it, and the land lock and the land tree follow only whether a land runs `[land] check`, no longer a caller's `CARGO_TARGET_DIR`. This is the contract step `20261108-toolchain-neutral-land-builds` left for later (`backlog/land-still-sets-cargo-variables`), whose condition the human judged met: `v0.23.0` ships `upgrade/migration-land-dir`, which already told projects a later `clm` stops setting the cargo variables, and `ci/land-dir` is current.

## The human's answers

> Question: its condition is met (v0.23.0 carries upgrade/migration-land-dir; ci/land-dir is current). Draft the expand-contract step that moves CARGO_TARGET_DIR/CARGO_INCREMENTAL out of clm land into the starter check script?
>
> Answer: "Draft it now (Recommended)".

## Problem

`land` still knows one toolchain. It sets `CARGO_TARGET_DIR` to `.clm/land` and `CARGO_INCREMENTAL=0` for every land check (`shared-land-target`), and a caller who sets `CARGO_TARGET_DIR` takes no land lock, gets no land tree and no `CLM_LAND_DIR` (`land-target-lock`, `land-tree`, `land-dir-env`), so whether a land is serialised and warm depends on a cargo variable in the lander's shell. A project in another language gets the place (`CLM_LAND_DIR`) but CLM still prescribes cargo's settings to every project. This repository's own check already points cargo at `CLM_LAND_DIR` (`ci/land-dir`); a project on the starter script doesn't.

## Approaches

- **Starter script and migration carry the cargo lines; `land` sets nothing (chosen).** The item's plan. A project that kept the starter script gets the lines on `clm upgrade --apply`; one that edited it gets a note saying what to add. Every land that checks takes the lock and the land tree, whatever the caller's environment.
- **Keep setting the variables for projects whose script doesn't name `CLM_LAND_DIR`.** Rejected: `land` would read the project's script to decide, and would keep knowing cargo, the coupling this removes.
- **Drop the cargo variables with no starter change, leaving it to the `v0.23.0` note.** Rejected: a project on the unedited starter script would lose warm lands silently; `upgrades-carry-migrations` wants the template change and its migration.
- **Two releases: the script and migration first, `land`'s change in a later change.** Rejected as unneeded: the expand step already shipped in `v0.23.0` (`CLM_LAND_DIR` and the advised note), and the requirements' `#### Needs` keep `land`'s contract step from being built before the starter script and its migration are, so one release carries all three. What a project loses until it takes the migration is warmth, never correctness.

## Outcome

- `land/shared-land-target`, modified: `clm land` leaves `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` as the caller has them and hands the place as `CLM_LAND_DIR`; the lock and the land tree follow only whether the land runs `[land] check`. Needs `init/starter-check-land-dir` and `upgrade/migration-starter-land-dir`.
- `land/land-target-lock`, `land/land-tree`, `land/land-target-cap`, `land/land-dir-env`, modified, each with the same `#### Needs`, since their code is one edit with `shared-land-target`'s (`target()` and its `shared` flag; builder check): no exemption for a caller's `CARGO_TARGET_DIR`; every land check gets `CLM_LAND_DIR`; the cap measures `.clm/land` after every locked land and never a caller's directory; scenarios write into `CLM_LAND_DIR` instead of `CARGO_TARGET_DIR`. `land`'s Design says the same.
- `init/starter-check-land-dir`, added: the Rust starter script exports `CARGO_TARGET_DIR` from `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` when `CLM_LAND_DIR` is set, before `cargo test`.
- `upgrade/migration-starter-land-dir`, added: an advised note migration, ID `20261109-land-still-sets-cargo`, `old` the `v0.16.0`–`v0.23.0` Rust starter script (identical in every corpus entry from `v0.16.0` to `v0.23.0`), `new` today's; `done` when the script is absent, names `CLM_LAND_DIR` or runs no `cargo`.
- `upgrade/migration-check-script-from-root`, modified: its `new` is the `v0.16.0` script, as its frozen text already is, no longer "what `clm init` writes today"; the `v0.14.0` script reaches today's text through both migrations in one `--apply`.
- The guide's `ci-and-deployment` page gains **Where your check builds**, naming `CLM_LAND_DIR`, the starter's lines and `target_cap_gb`.
- `orchestrator.run/template-migration-fix`, added: `clm run`'s prompt tells a build whose requirement changes a file `clm init` writes to write the `upgrade` migration an `upgrade` requirement states for it, citing nothing of `upgrade`'s, and to name it under `## Fixes in other components`.
- `ARCHITECTURE.md`'s `path-ownership` gains that fourth exception, with a scenario, enforced by the new prompt requirement; `fixed-land-tree` and `path-ownership-by-prompt` are amended; `template-change-ships-its-migration` is recorded.

## Constraints

- `ci` needs no change: `.github/scripts/ci-check.sh` already exports both cargo variables from `CLM_LAND_DIR` (`ci/land-dir`), so this repository's lands stay warm when `land`'s requirement is built.
- `clm land --check-only` is unchanged: it never set the cargo variables and still removes `CLM_LAND_DIR` (`check-only`, `land-dir-env`).
- `upgrades-carry-migrations` and `shipped-migrations-frozen`: the new migration ships with the next release and is locked then; no shipped migration's files change.
- `expand-contract`: nothing is marked `clm:remove-by`, since no caller's code uses the variables `land` set; the expand step was `CLM_LAND_DIR` and the `v0.23.0` note.

**Edge cases and workarounds known now.**
- `clm/upgrade-corpus` fails on `main` if the starter template and its migration land in separate builds, in either order: the template alone has no migration, and the migration alone rewrites the script to a text `clm init` doesn't write yet. So `init/starter-check-land-dir`'s build writes `upgrade`'s migration too, and fixes `upgrade`'s two tests of `migration-check-script-from-root` that the new migration makes fail (`shipped_tests.rs`: the 0.14 rewrite now ends at today's text, and the `v0.16.0` script is no longer up to date; builder check), and `upgrade/migration-starter-land-dir` needs it, its build adding the migration's tests (`template-change-ships-its-migration`). This is the one edit outside a component's `paths` the change asks for, and `path-ownership`, a carried `method` constraint, gains it as a fourth exception (architecture audit), so the branch needs review.
- A project whose script is not the unedited Rust starter (edited, or several languages) isn't rewritten; the note tells it what to add. A script that runs cargo only through another script it calls holds no `cargo` and so never sees the note; accepted, since the `v0.23.0` note already named the variables to every project.
- The starter's Rust entry becomes several lines (the export and `cargo test`), and the migration's `new` must equal what `clm init --lang rust` writes byte for byte; the corpus test guards it.
- A lander who exports `CARGO_TARGET_DIR` in their shell now passes it to the check unchanged; the starter script and `ci-check.sh` override it from `CLM_LAND_DIR` during a land, and a project script that doesn't builds where the caller says, outside the cap.

## Decisions

- `template-change-ships-its-migration` (new): a change to a file `clm init` writes lands in one build with the migration that carries it; `init`'s build writes the migration, `upgrade`'s proves it. It amends `path-ownership` (the architecture audit found that a decision alone can't widen a carried constraint) and refines `path-ownership-by-prompt`.
- `fixed-land-tree`, amended: a caller's `CARGO_TARGET_DIR` no longer exempts a land from the lock and the land tree.
- The migration is a note with `old`/`new`, not an apply migration as the item put it: `upgrade`'s apply migrations exist only for exact rewrites of any text, and a check script the project may have edited is rewritten only while it equals the released text, as `migration-check-script-from-root` does.

## Open questions

- None yet.
