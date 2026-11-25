---
title: "`init`'s first migration of its own breaks `upgrade/migration-list`'s *a fresh project is done*"
kind: question
components: [init, upgrade]
blocks: [init/migration-group-markers, init/all-migrations]
choices:
  - "Judge it over init's list elsewhere"
  - "Reword it for a stamp upgrade lacks"
---

`init/migration-group-markers` (`bd95320b`) adds `20261124-a-marker-above-a`, `init`'s first migration of its own, and `init/all-migrations` (`da1da9d3`) names it last in `migrations()`. Built as stated (an `advised` note over `clm.toml`, `done` reading the stamp against `clm_init::migrations()`), every `clm-init` test passes, but `upgrade`'s `crates/clm-upgrade/tests/corpus.rs::a_fresh_project_has_every_migration_done` fails, and it is `upgrade`'s file, which `init`'s build may not change (`path-ownership`).

That test proves `upgrade/migration-list`'s scenario *a fresh project is done*: on a repository the built `clm init` has just set up, every `done` of `upgrade`'s own `migrations()` returns true but that of `20261105-clm-upgrade-2`, and `pending_in(clm_upgrade::migrations(), clm_upgrade::supersedes(), root)` returns nothing. Once `init` holds a migration, `clm init` stamps `upgraded` with it (`init/init-stamps-upgraded`), so:

- the `done` of `upgrade`'s four stamp-taken notes (`20261107-projects-upgrading-past-the`, `20261108-toolchain-neutral-land-builds`, `20261109-a-build-s-tests`, `20261114-proof-is-real`) looks for that stamp in `upgrade`'s own list, finds none, and returns false, which `upgrade/pending-migrations` already says and handles in `pending_in` (`stamp-taken-notes`);
- `pending_in` over `upgrade`'s list alone returns `TooNew` for a stamp that list doesn't hold, as `pending-migrations` states.

Every real caller passes `clm_init::migrations()`, so nothing a person runs is wrong: only the scenario, which judges a fresh project over `upgrade`'s list alone, can't hold once `init` has a migration. `upgrade` can't read `init`'s list (`init` depends on `upgrade`). A spec change should move or reword the scenario before `init`'s build: for example state it where `init`'s list is in reach (the root's or `init`'s own tests, judging `pending_in(clm_init::migrations(), clm_init::supersedes(), root)`), or reword `upgrade`'s to exclude the four stamp-taken notes and to judge `pending_in` over `upgrade`'s list followed by the stamped ID. Then `init`'s build adds the migration and re-cites `all-migrations`' five tests.
