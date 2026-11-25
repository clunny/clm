---
title: "`init`'s first migration of its own breaks `upgrade/migration-list`'s *a fresh project is done*"
kind: question
components: [init, upgrade]
blocks: [init/migration-group-markers, init/all-migrations]
choices:
  - "Move it to init, over init's list"
  - "Let upgrade's test use clm-init"
  - "Narrow it to upgrade's own notes"
---

`init/migration-group-markers` (`bd95320b`) adds `20261124-a-marker-above-a`, `init`'s first migration of its own, and `init/all-migrations` (`da1da9d3`), whose scenario *the list today* names it last in `migrations()`. Built as stated (an `advised` note over `clm.toml`, `done` reading the stamp against `clm_init::migrations()`), every `clm-init` test passes, but `upgrade`'s `crates/clm-upgrade/tests/corpus.rs::a_fresh_project_has_every_migration_done` fails, and it is `upgrade`'s file, which `init`'s build may not change (`path-ownership`).

That test proves `upgrade/migration-list`'s scenario *a fresh project is done*: on a repository the built `clm init` has just set up, every `done` of `upgrade`'s own `migrations()` returns true but that of `20261105-clm-upgrade-2`, and `pending_in(clm_upgrade::migrations(), clm_upgrade::supersedes(), root)` returns nothing. Once `init` holds a migration, `clm init` stamps `upgraded` with it (`init/init-stamps-upgraded`), so:

- the `done` of `upgrade`'s four stamp-taken notes (`20261107-projects-upgrading-past-the`, `20261108-toolchain-neutral-land-builds`, `20261109-a-build-s-tests`, `20261114-proof-is-real`) looks for that stamp in `upgrade`'s own list, finds none, and returns false, which `upgrade/pending-migrations` already says and handles in `pending_in` (`stamp-taken-notes`);
- `pending_in` over `upgrade`'s list alone returns `TooNew` for a stamp that list doesn't hold, as `pending-migrations` states.

Every real caller passes `clm_init::migrations()`, so nothing a person runs is wrong: only the scenario, which judges a fresh project over `upgrade`'s list alone, can't hold once `init` has a migration. `upgrade`'s `depends_on` is `[config, vcs]`, and `init` depends on `upgrade`, so `upgrade`'s files can't read `init`'s list today. A spec change should settle it before `init`'s build, one of:

1. Move the scenario to where `init`'s list is in reach (`init`'s own tests, or the root's), judging every `done` and `pending_in(clm_init::migrations(), clm_init::supersedes(), root)` on a fresh project. It then keeps proving the whole list is done.
2. Keep it in `upgrade` and let its integration test take `clm-init` as a dev-dependency (Cargo allows that cycle), which needs `upgrade`'s `depends_on` or an exemption to allow the import (`check/undeclared-imports`).
3. Narrow `upgrade`'s scenario to what `upgrade` alone can show: the four stamp-taken notes left out of the `done` check, and `pending_in` over `upgrade`'s list followed by a migration carrying the stamped ID. This proves less: nothing then shows a fresh project has `init`'s own migrations done, unless `init`'s `migration-group-markers` *a new project* stands for it.

Then `init`'s build adds the migration and re-cites `all-migrations`' five tests, which stay `behind` until then since *the list today* names the migration.
