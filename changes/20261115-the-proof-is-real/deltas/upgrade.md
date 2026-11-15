---
component: upgrade
---

## MODIFIED Requirements

### Requirement: migration-markers-in-tests <!-- base: b7a52f4d3ab4ead1 -->
An `advised` note migration, ID `20261114-proof-is-real`, SHALL tell a project set up before a marker counted only inside a test (`check.coverage/markers-in-tests`) what changes for it. It edits no file: its `file` SHALL be `clm.toml`, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in the crate's own `migrations()`, and false otherwise; for a stamp that names a later migration of `init`'s, `pending_in` leaves this note out (`pending-migrations`), so a project `clm init` set up since is done, and an older one is done once `clm upgrade --apply` has stamped it (`stamp-taken-notes`). Its note's first line SHALL read `A marker now counts only inside a test: see what stops counting before the next build`, and the note SHALL name `clm coverage`, whose `uncounted` lines list the markers that stopped counting, `wiring`, the root key that switches the ownership rule on, and `[verify] mutate`, the setting of the mutation pass, both off until the project sets them. Of the ownership rule it SHALL say that `clm land` refuses a build's branch, `impl/<component>`, that changes a file the rule doesn't count as its own (`land/build-holds-own-files`), which no review lets through, so the note holds before and after `gate/own-files` drops the clause "a file that cites the component", and that a project whose tests share files first gives each test file to its component with a `tests` glob (`model/test-ownership-keys`) before it names `wiring`. It SHALL NOT say that such a branch needs review.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261114-proof-is-real advised note A marker now counts only inside a test: see what stops counting before the next build`, the second prints `note 20261114-proof-is-real` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261114-proof-is-real` runs on a project it is pending in
- THEN the note names `clm coverage`, `uncounted`, `wiring`, `clm land` refuses, `tests` glob and `[verify] mutate`, and doesn't hold `needs review`

#### Scenario: a new project
- GIVEN a project `clm init` has just set up
- WHEN `clm upgrade` runs
- THEN it prints `up to date`

#### Needs
- check.coverage/markers-in-tests
- check/coverage-lists
- config/verify-setting
