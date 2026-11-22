---
status: applied
resolves: [verify-env-s-bisect-case-can-t-hold-with-bad-as-origin-main-s]
---

## Intent

`verify/verify-env`'s scenario "a table that can't be used" states its fourth case so that it can hold: the fixture adds, after `bad`, a land that restores `g`'s `clm.toml`, so `bad` is no longer `origin/main`'s head and `settings-from-main`'s load of `origin/main:clm.toml` doesn't refuse before the bisect's first probe.

## Problem

Case 4 builds a passing record for `g`, a land `l1`, then `bad` (whose `[env]` `config/env-setting` refuses) and a red record for `bad`, and runs `clm verify --bisect <bad>`. As written `bad` is `origin/main`'s head, and the requirement itself says that for `origin/main`'s head `settings-from-main`'s load refuses first, so the call exits 1 before any probe and the case's outcome can't happen. The test (`crates/clm-verify/tests/env_commits.rs::a_bisect_of_a_red_commit_whose_table_cant_be_used`) already adds such a later land; the scenario didn't say so.

## Approaches

- **Taken: state the later land in the scenario's GIVEN/WHEN text.** The requirement is right and the code does what it says; only the scenario's fixture was incomplete. The scenario keeps its name.
- **Not taken: a requirement that bisect loads settings from the red commit rather than `origin/main`.** It would widen `settings-from-main` for a fixture's sake, with nothing a user needs from it (`yagni`).

## Outcome

`verify-env`'s requirement text is unchanged; its scenario's fourth case names the restoring land. The requirement's version moves, so `verify`'s build re-cites its tests at the new version; no test changes what it asserts, and none is deleted.
