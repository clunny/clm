---
component: check.coverage
base: { contract: "448a6265d60f2e28" }
---

## MODIFIED Contract
How far the code has caught up with the spec, for `check` and, through `clm-check`'s re-exports, for every component that reads it: the queue (`orchestrator`), the views (`view`), `clm rebase` and the `web` Worker (`check-coverage-split`). The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library, which `clm-check` re-exports:
- `markers_in(path, text)`: the `req:` markers in one file, none for Markdown; `scan(root)`, with the default feature `checkout`: the `req:` and `clm:remove-by` markers of every file a checkout lists, as `Scan { req, remove_by }`.
- `report(graph, markers)`: each requirement's `Coverage`, human-layer entries included (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements (`pending-report`).
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)` and `complete_changes`, built on `change_progress` (`status-report`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` glob matches a repository-relative path, as `pending` and `check`'s `paths-disjoint` read `paths`.
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`); `check.affected` reads it through `clm-check` (`check/path-ownership`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## ADDED Requirements

### Requirement: removed-capabilities
A capability SHALL count as *removed* while a change outside the archive, whatever its status, lists its slug under `removes_capabilities` (`model/parse-changes`) and `CAPABILITIES.md` holds no entry of that slug: `removed(graph)` SHALL give `capability/<slug>` with the IDs of those changes, sorted, as it gives a requirement a delta removed, and SHALL NOT give a listed capability that `CAPABILITIES.md` holds, which is a capability like any other. A marker citing a removed capability SHALL read as `obsolete`, as one citing a removed requirement does: `clm coverage` prints `capability/<slug> obsolete <file>:<line>[ ...]` among its obsolete lines (`coverage-report`), and `clm pending` prints the same line, with every marker's location, where `pending-report` places an obsolete requirement, which is the component `capability` when the graph holds it, and otherwise the owner of each marker's file, or the root. Nothing blocks the line, since no item, need, move or serving rule can name a capability that is gone. So a spec branch that removes a cited capability by hand passes `clm check` (`check/dangling-markers`), and the build of the component the line is pending on removes the tests (`removed-capabilities-build-out`). Once the change is archived, the capability is no longer removed and its markers dangle, so the change can't be complete while one is left (`status-report`).

#### Scenario: removed capability, cited
- GIVEN component `capability`, a `CAPABILITIES.md` with no entry `gone`, an `applied` change `20260101-x` with `removes_capabilities: [gone]`, and markers citing `capability/gone` at `crates/clm/tests/capability/gone.rs:3` and `capabilities/web/gone.spec.ts:5`
- WHEN `removed`, `clm coverage` and `clm pending capability` run
- THEN `removed` gives `capability/gone` with `[20260101-x]`, the last line of `clm coverage` is `capability/gone obsolete capabilities/web/gone.spec.ts:5 crates/clm/tests/capability/gone.rs:3`, and `clm pending capability` prints the same line, with nothing after it

#### Scenario: listed but still held
- GIVEN a `CAPABILITIES.md` holding `gone`, cited at its current version, and a `proposed` change with `removes_capabilities: [gone]`
- WHEN `removed` and `clm coverage` run
- THEN `removed` gives nothing, and `clm coverage` prints `capability/gone current …`

#### Scenario: change archived
- GIVEN the first scenario's files, with change `20260101-x` under `changes/archive/`
- WHEN `removed` and `clm pending` run
- THEN `removed` gives nothing, and `clm pending` prints no line for `capability/gone`

#### Scenario: no capability component
- GIVEN root `clm`, no component `capability`, component `v` with `paths` `["crates/v/**"]`, a `CAPABILITIES.md` with no entry `gone`, an `applied` change with `removes_capabilities: [gone]`, and markers citing `capability/gone` at `crates/v/t.rs:4` and `old/t.rs:2`, which no component's `paths` match
- WHEN `clm pending` runs
- THEN it prints `capability/gone obsolete old/t.rs:2` for `clm` and `capability/gone obsolete crates/v/t.rs:4` for `v`

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- model/parse-changes

## MODIFIED Requirements

### Requirement: status-report <!-- base: 47ccc387510c6008 -->
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds, nor a capability it lists under `removes_capabilities` that `CAPABILITIES.md` no longer holds (`removed-capabilities`). For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. A section with no rows has the single row `none`. It SHALL exit 0, or print the violations and exit 1 when the spec graph has violations.

#### Scenario: entries on the root's row
- GIVEN root `clm` with no requirements, `architecture/a` enforced by a current rule of another component, `capability/b` cited by nothing, `capability/c` cited at its current version, and `capability/u` holding `#### Untested`
- WHEN `clm status` runs
- THEN the `clm` row reads `1/2`

#### Scenario: one pending change
- GIVEN component `a` (active, requirements `a/x` current and `a/y` not), and change `20260101-c` with status `applied` whose delta adds `a/x` and `a/y`
- WHEN `clm status` runs
- THEN its `components` section has a row with the cells `a`, `active` and `1/2`, and its `changes` section has a row with the cells `20260101-c` and `1/2 requirements` in the `applied` group

#### Scenario: requirement removed later
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, `a/x` current, and no component holding `a/y`
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `1/1 requirements` and `complete`

#### Scenario: component renamed later, not yet built
- GIVEN an `applied` change `20260101-c` whose delta for `g` adds `g/x`, no component `g`, an `applied` change `20260102-r` whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements` and whose `create: true` delta for `v` adds `v/x`, and `v/x` uncited
- WHEN `clm status` runs
- THEN the row for `20260101-c` has the cells `20260101-c` and `0/1 requirements`, and no `complete` cell

#### Scenario: component renamed later, built
- GIVEN the changes and components of "component renamed later, not yet built", with `20260102-r` archived as `done`, and `v/x` cited at its current version
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `1/1 requirements` and `complete`

#### Scenario: renamed twice
- GIVEN an `applied` change `20260101-c` whose delta for `g` adds `g/x`, no components `g` or `v`, an `applied` change `20260102-r` that moves `g/x` to `v/x`, an `applied` change `20260103-s` that moves `v/x` to `w/x`, and `w/x` cited at its current version
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `1/1 requirements` and `complete`

#### Scenario: requirement added back later
- GIVEN an `applied` change `20260101-c` whose delta removes `a/x`, and component `a` holding a requirement `a/x` that a test cites at its current version
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `0/0 requirements` and `complete`

#### Scenario: removed requirement still cited
- GIVEN an `applied` change `20260101-c` whose delta removes `a/x`, no component holding `a/x`, and a test citing `a/x`
- WHEN `clm status` runs
- THEN the row for `20260101-c` has no `complete` cell

#### Scenario: removed capability still cited
- GIVEN an `applied` change `20260101-c` with no deltas and `removes_capabilities: [gone]`, a `CAPABILITIES.md` with no entry `gone`, and a test citing `capability/gone`
- WHEN `clm status` runs
- THEN the row for `20260101-c` has the cells `20260101-c` and `0/0 requirements`, and no `complete` cell; once the test is deleted, the row has a `complete` cell

#### Scenario: groups in order
- GIVEN change `20260101-a` with status `applied` and change `20260101-z` with status `implementing`
- WHEN `clm status` runs
- THEN the `implementing` group, holding `20260101-z`, comes before the `applied` group, holding `20260101-a`

#### Scenario: unapplied change
- GIVEN a `proposed` change `20260101-p` whose delta adds `a/z`, which no component holds yet
- WHEN `clm status` runs
- THEN the row for `20260101-p` has the cells `20260101-p` and `0/1 requirements`, and no `complete` cell

#### Scenario: nothing in flight
- GIVEN no changes outside the archive
- WHEN `clm status` runs
- THEN the `changes` section has the single row `none`

#### Serves
- capability/see-project-state
- capability/watch-beside-chat
- capability/wrap-up-session
- architecture/component-reconciliation

#### Needs
- model/human-layers
- model/reserved-ids
- plugin.hooks/session-status
- model/parse-changes

### Requirement: change-progress <!-- base: dac448cb879881bd -->
`change_progress(graph, markers)` SHALL return, for each change outside the archive, in ID order, its ID; `current` and `total`, its requirements current out of the requirements its deltas add or modify; `obsolete`, the markers (`<file>:<line>`) citing a requirement its deltas remove that the spec no longer holds, or a capability it lists under `removes_capabilities` that `CAPABILITIES.md` no longer holds (`removed-capabilities`); and whether it is `complete`, all counted as `check.coverage/status-report` counts them. For every change, the row `clm status` prints SHALL carry the same `<current>/<total>` and the same `complete`, and `complete_changes` SHALL name exactly the changes it marks complete.

#### Scenario: same counts as status
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, `a/x` current and `a/y` cited at an older version, and an `applied` change `20260101-d` whose two added requirements are both current
- WHEN `change_progress` runs on the graph and markers that `clm status` reads
- THEN it gives `20260101-c` 1 of 2 current and not complete and `20260101-d` 2 of 2 and complete, `clm status` prints `1/2 requirements` and `2/2 requirements` with `complete` only on `20260101-d`, and `complete_changes` returns `20260101-d` alone

#### Scenario: removed requirement still cited
- GIVEN an `applied` change `20260101-c` whose delta removes `a/x`, no component holding `a/x`, and a test citing `a/x` at `t.rs:3`
- WHEN `change_progress` runs
- THEN `20260101-c` has `obsolete` `t.rs:3` and is not complete

#### Scenario: removed capability still cited
- GIVEN an `applied` change `20260101-c` with no deltas and `removes_capabilities: [gone]`, a `CAPABILITIES.md` with no entry `gone`, and a test citing `capability/gone` at `t.rs:3`
- WHEN `change_progress` runs
- THEN `20260101-c` has `obsolete` `t.rs:3`, 0 of 0 current, and is not complete

#### Serves
- capability/see-project-state
- capability/wrap-up-session
- architecture/component-reconciliation

#### Needs
- model/parse-changes
