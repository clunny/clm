---
component: upgrade
---

## ADDED Requirements
### Requirement: migration-land-dir
An `advised` note migration, ID `20261108-toolchain-neutral-land-builds`, SHALL tell a project set up before it that lands now hand its check a build place of its own and check only where a branch and `main` overlap. It edits no file: its `file` SHALL be `clm.toml`, where `[land] check` names the check, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()`, and false otherwise, as `migration-tests-overlap`'s is. The fixture `migration-tests-overlap` names for an up-to-date project SHALL stay stamped with `latest()`. Its note's first line SHALL read `Lands hand your check CLM_LAND_DIR and re-check only the overlap`. The note SHALL say:
- that `clm land` sets `CLM_LAND_DIR` for a check it runs in the clone's land tree, a place that lasts from land to land, and that a project whose check builds should point its toolchain's output there in its own check script (for cargo, `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`), since a later `clm` stops setting the cargo variables itself;
- that a land whose branch's head was checked whole by `clm land --check-only` now runs only the tests both the branch and what `main` gained since can affect, and none when there are none, so its verdict rests on the project's tests declaring what they read (`land-checks-stay-fast`, which `clm guide constraints` states): an owner and `depends_on`, `req:` markers, and `clm:reads` markers for the files they read;
- that tests that are slow, flaky or need a resource no other run may share belong in `[verify]`, after the land, and that a failure there is reported with how the culprit's land re-checked, so the project can judge whether a fact, the code or the test was wrong;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261108-toolchain-neutral-land-builds advised note Lands hand your check CLM_LAND_DIR and re-check only the overlap`, the second prints `note 20261108-toolchain-neutral-land-builds` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261108-toolchain-neutral-land-builds` runs on a project it is pending in
- THEN the note names `CLM_LAND_DIR`, `CARGO_TARGET_DIR`, `land-checks-stay-fast`, `clm guide constraints`, `clm:reads` and `[verify]`

#### Needs
- land/land-dir-env
- land/overlap-recheck
- clm/upgrade-command
