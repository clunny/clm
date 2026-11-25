---
component: init
---

## MODIFIED Requirements

### Requirement: all-migrations <!-- base: 2cec4a46f460a651 -->
`clm-init` SHALL offer `migrations()`, every migration the binary holds: `upgrade`'s own (`upgrade/own-list-closed`) in their order, followed by this crate's own in the order they landed; `own_migrations()`, this crate's alone, which the lock step reads; and `supersedes()`, `upgrade`'s `supersedes()` followed by this crate's pairs. No two migrations SHALL share an ID, and none of this crate's SHALL be `required` with a `note`.

#### Scenario: the list today
- GIVEN the crate
- WHEN `migrations()` and `supersedes()` are read
- THEN they are `upgrade`'s thirteen in their order followed by `20261124-a-marker-above-a` (`migration-group-markers`), and `upgrade`'s pairs

#### Scenario: a migration of its own
- GIVEN `upgrade`'s list followed by one migration `z`, built as `migrations()` builds its list
- WHEN it is read, and `upgrade`'s `pending_in` runs over it for a project whose `upgraded` names the last of `upgrade`'s and for which `z` is not done
- THEN `z` is last, and it is pending

#### Scenario: a duplicate ID
- GIVEN the binary's list
- WHEN `migrations()` and `own_migrations()` are read
- THEN no two migrations share an ID, and none of `own_migrations()` is `required` with a `note`

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations

#### Needs
- upgrade/migration-list
- upgrade/pending-migrations
- upgrade/own-list-closed

## ADDED Requirements

### Requirement: migration-group-markers
An `advised` note migration of this crate, ID `20261124-a-marker-above-a`, SHALL tell a project set up before a marker on a group of tests' own lines stopped counting (`check.coverage/markers-in-tests`) what changes for it. It SHALL edit no file: `clm upgrade --apply` changes nothing for it but `clm.toml`'s `upgraded` line. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()` (`all-migrations`), and false otherwise, so a project `clm init` sets up from now on, which it stamps with the last of `migrations()`, is done, and an older one is done once `clm upgrade --apply` has stamped it (`upgrade/pending-migrations`). While it is pending, `clm upgrade` SHALL list it as `20261124-a-marker-above-a advised note A marker above a describe block or a Test class no longer counts: put one on each test`, and `clm upgrade --note 20261124-a-marker-above-a` SHALL print a note saying that a `req:` marker on a group's own lines (a `describe` or `test.describe` block, a Python `Test` class, the comment directly above one included) proves nothing for the tests in it, that a requirement cited only so reads `unbuilt` and a build brings it up to date, that `clm coverage`'s `uncounted` lines list each such marker with its file, and that a marker moved onto each test that proves the requirement counts again, after which `clm health` calls none of those tests uncited and `clm land` refuses the deletion of each unless a `deleted-tests.txt` line names it.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to `20261114-proof-is-real`, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261124-a-marker-above-a advised note A marker above a describe block or a Test class no longer counts: put one on each test`, the second prints `note 20261124-a-marker-above-a` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261124-a-marker-above-a` runs on a project it is pending in
- THEN the note names `describe`, `Test` class, `unbuilt`, `clm coverage`, `uncounted`, `clm health`, `clm land` and `deleted-tests.txt`

#### Scenario: a new project
- GIVEN a directory `clm init` has just set up
- WHEN `clm upgrade` runs
- THEN it prints `up to date`

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations

#### Needs
- check.coverage/markers-in-tests
- upgrade/pending-migrations
