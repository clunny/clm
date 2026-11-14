---
title: "A template clm init writes and the upgrade migration that carries its change live in different components"
kind: idea
components: [init, upgrade]
---

## Intent

A requirement that changes a file `clm init` writes must ship the `upgrade` migration in the same land, because the upgrade corpus test fails a land of either half alone (`template-change-ships-its-migration`). The two halves live in different components. Until 2026-11-14 `init`'s build wrote into `upgrade`'s files; since `20261114-proof-switched-on` the land refuses that (`land/build-holds-own-files`), so such a requirement waits behind a question item until this is built. Things that must land together belong in one component.

## Settled

- **They share a component, and no rule keeps the exception** (2026-11-14, with the human, `own-files-at-the-gate`). Naming the migration files in the root's `wiring`, and an exemption rule, were both rejected.
- **The migrations of the files `clm init` writes live with those files, in `init`.** `upgrade` keeps the machinery that lists, applies, notes, declines and locks them.

## Approaches

- **Forward only.** New migrations of `init`'s files are written in `init`. The ones a release has shipped stay where they are: they are frozen (`upgrade/shipped-migrations-frozen`), so which component owns them never matters. `upgrade`'s commands are given the list of migrations by their caller, which avoids the cycle `template-change-ships-its-migration` names, since `init` already depends on `upgrade`.
- **Move them all.** Every migration of an `init` file moves to `init` with its requirement. It moves frozen files and renames requirements for no gain over the first.

## Open questions

- `upgrade`'s tests of one migration apply every migration, so they fail when another is added: all four `init` builds that wrote a migration also mended them, in `crates/clm-upgrade/src/shipped_tests.rs` and `crates/clm/tests/upgrade.rs`. Each test should apply only the migrations it is about.
- Who gives `upgrade` the list: the root's dispatch, or `init`, which stamps `upgraded` with the latest ID?
- Does the lock (`clm upgrade --lock`) read both places?
