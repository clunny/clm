---
title: Coverage
status: active
owners: ["@clunny"]
depends_on: [model, vcs]
paths: ["crates/clm-coverage/**"]
successor: ""
serves: [capability/prove-by-tests, architecture/proven-by-tests, architecture/hash-based-staleness]
---

## Contract
How far the code has caught up with the spec, for `check` and, through `clm-check`'s re-exports, for every component that reads it: the queue (`orchestrator`), the views (`view`), `clm rebase` and the `web` Worker (`check-coverage-split`). The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library, which `clm-check` re-exports:
- `markers_in(path, text)`: the `req:` markers in one file, none for Markdown; `scan(root)`, with the default feature `checkout`: the `req:` and `clm:remove-by` markers of every file a checkout lists, as `Scan { req, remove_by }`.
- `report(graph, markers)`: each requirement's `Coverage`, human-layer entries included (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements (`pending-report`).
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)` and `complete_changes`, built on `change_progress` (`status-report`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` glob matches a repository-relative path, as `pending` and `check`'s `paths-disjoint` read `paths`.
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`); `check.affected` reads it through `clm-check` (`check/path-ownership`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`), which the wasm build leaves off.

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). `pending(graph, markers)` gives each requirement its state from the markers, and each component its pending requirements: the `behind` and `unbuilt` ones among its requirements, and the `obsolete` ones whose ID names the component. A requirement's priority is the highest among the changes outside the archive whose deltas add, modify or remove it, `P5` when none does. A backlog item *blocks* the requirements its `blocks` lists (`requirement-blocks`), and a pending requirement *blocks* the requirements whose `Needs` name it (`requirement-needs`); the list carries both, items as slugs in `blocked_by` and everything else a requirement waits on as `<component>/<slug>` in `waits_on`, so the queue needs nothing else. A current requirement blocks nothing, whatever names it. An `obsolete` requirement that a move took to requirements the spec holds is blocked, in `waits_on`, by each of those that is pending (`moved-markers-wait`): the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands. A pending capability is blocked, in `waits_on` too, by each `unbuilt` requirement whose effective serves (`Graph::effective_serves`, `model/serves`) names it (`serving-rules-order-capabilities`): its test exercises those rules, so it waits until each has been built once. A `behind` rule doesn't hold it, so an edit to an old rule leaves an existing capability's test free to follow its own edit. The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn: a `Needs` can't name an entry (`model/requirement-needs`), and the rules a capability waits on are never entries. `clm pending` prints it for the working tree.

**Progress** (`change-progress`). `change_progress(graph, markers)` computes the coverage once and returns one `ChangeProgress { change, current, total, obsolete, complete }` per change outside the archive, in ID order, whatever its status; a caller that shows only some statuses filters them. `obsolete` lists its markers in the order of `markers`. `status_report`'s change rows and `complete_changes` are built on it, and `view`'s `flight` model takes every count of a change from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change.

**Moves** (`progress-follows-moves`). The IDs of an `applied` or `implementing` change that the spec doesn't hold are mapped through the *moves* the graph's changes record, as `status-report` defines them. A rename (`node-removal`) moves every requirement of the old component. Only `ADDED Requirements` make a move, `MODIFIED` ones don't, and neither delta needs `create: true` or `remove: true`, so a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The spec holds an ID when a component holds a requirement with it; an ID the spec doesn't hold is followed whatever its coverage state, `obsolete` included, and an ID the spec holds is never followed further. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, an ID that no move takes anywhere the spec holds is left out, as a removed requirement is, and the counted IDs are deduplicated after the mapping. A split can make `total` larger than the number of requirements the deltas list. `pending` reads the same map, through the same breadth-first walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves, and a change's `obsolete` markers are those citing what it removed.

## Requirements

### Requirement: coverage-report
`clm coverage` SHALL print one line per requirement, and per human-layer entry (`architecture/<slug>`, `capability/<slug>`, `model/human-layers`), all sorted together by ID: `<id> current <file>:<line>[ <file>:<line>...]` when markers cite it and every one cites its current version, `<id> behind <file>:<line>[ ...]` listing the markers that cite it at another version or without one when any does, even beside markers at its current version, which the line leaves out (`stale-markers-keep-behind`), otherwise `<id> unbuilt`. An architecture entry is never cited (`check/dangling-markers`): its line is `<id> current` when it holds an `#### Enforced by` block and every rule it names is current, `<id> pending <rule>[,<rule>...]` naming, in block order, the rules it names that aren't, `<id> untested` when it holds only an `#### Untested` block, and `<id> unenforced` when it holds neither, which `check/unenforced` fails; a capability holding `#### Untested` is `<id> untested` (`human-layer-proof`); and, after those, one line `<id> obsolete <file>:<line>[ <file>:<line>...]` per requirement that no longer exists but that a change outside the archive removed and a marker still cites. It SHALL exit 0.

#### Scenario: mixed coverage
- GIVEN `a/x` at version `aaaaaaaa` cited at `t.rs:2` as `req:a/x@aaaaaaaa`, `a/y` cited at `t.rs:4` as `req:a/y@bbbbbbbb` while its version is `cccccccc`, `a/z` cited at `t.rs:6` without a version, and `a/w` uncited
- WHEN `clm coverage` runs
- THEN it prints `a/w unbuilt`, `a/x current t.rs:2`, `a/y behind t.rs:4` and `a/z behind t.rs:6`

#### Scenario: stale marker beside a current one
- GIVEN `a/x` at version `aaaaaaaa` cited at `crates/a/t.rs:2` as `req:a/x@aaaaaaaa` and at `crates/b/t.rs:7` as `req:a/x@bbbbbbbb`
- WHEN `clm coverage` runs
- THEN it prints `a/x behind crates/b/t.rs:7`

#### Scenario: entries
- GIVEN `capability/b` cited at another version, and `capability/c` cited by nothing
- WHEN `clm coverage` runs
- THEN it prints `capability/b behind …` and `capability/c unbuilt`, sorted with the rules by ID

#### Scenario: architecture entries
- GIVEN `architecture/a` enforced by `n/x` and `n/y`, both current, `architecture/b` enforced by `n/x` and `n/z`, `n/z` unbuilt, `architecture/c` holding only `#### Untested`, and `capability/d` holding `#### Untested`
- WHEN `clm coverage` runs
- THEN it prints `architecture/a current`, `architecture/b pending n/z`, `architecture/c untested` and `capability/d untested`

#### Scenario: obsolete marker
- GIVEN `a/gone` cited at `t.rs:5`, removed by an `applied` change `x`
- WHEN `clm coverage` runs
- THEN its last line is `a/gone obsolete t.rs:5`

#### Needs
- model/human-layers

### Requirement: markers-from-files
For any checkout, `report` over the union of `markers_in` applied to each tracked file SHALL equal the coverage `clm coverage` reports, versions included, and `markers_in` SHALL return nothing for a Markdown file.

#### Scenario: same as checkout
- GIVEN a checkout with a test file citing one requirement at its current version and another at an older one, and a Markdown file citing a third
- WHEN coverage is computed with `markers_in` and `report` from the files' paths and texts, and with `clm coverage`
- THEN both report the first requirement current and the second behind at the same locations, and the third unbuilt

#### Serves
- capability/follow-from-browser
- capability/prove-by-tests
- architecture/git-is-truth

### Requirement: pending-report
`clm pending [<component>]` SHALL print one line per pending requirement of every component, or of `<component>`, sorted by component ID then requirement ID: `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` with the versions its markers cite other than its current one (`none` for a marker without one), sorted, or `<id> obsolete <file>:<line>[ ...]`; each followed by ` blocked <blocker>[,<blocker>...]` when it is blocked, each blocker `backlog/<slug>` for a backlog item that names it in `blocks` or `<component>/<slug>` for a pending requirement its `Needs` name (`requirement-needs`), for an `obsolete` one a pending requirement a move took it to, or for a capability an `unbuilt` requirement that serves it, items first, each group sorted. An `obsolete` requirement SHALL be blocked by each pending requirement that the moves `status-report` defines take it to, followed as `progress-follows-moves` follows them to the IDs the spec holds, on whatever component each is (`moved-markers-wait`): the build of that requirement re-points the markers, so the obsolete line waits for it. A capability that is pending SHALL count as pending on the component that builds the capabilities' tests, which `pending` takes from the model's `Graph::capability_tests_node` (`model/capability-tests-component`) rather than deciding itself, and is blocked like any requirement by the items whose `blocks` name it. A pending capability, `unbuilt` or `behind`, SHALL also be blocked by each `unbuilt` requirement whose effective serves (`model/serves`) names it, on whatever component, optional or not, and by no `behind` or current one (`serving-rules-order-capabilities`), so its test is built after the rules it exercises. A capability holding `#### Untested` is never pending, and neither is an architecture entry: the rules its `#### Enforced by` names are pending on their own components (`human-layer-proof`). It SHALL exit 0, printing nothing when nothing is pending, or print the violations and exit 1 when the spec graph has violations. An `obsolete` marker whose ID names a component that no longer exists, since a change removed it (`node-removal`), SHALL count as pending on the component whose `paths` match the marker's file, matched as git `:(glob)` pathspecs are, the deepest such component and then the first by ID when more than one does, or on the root `clm` when none does. This holds for every obsolete marker whose component is missing, however the component went. A removed requirement cited in files of several components is pending on each of them, each line listing only that component's files. `pending(graph, markers)` SHALL return the same for a graph and markers loaded without a checkout, with each requirement's priority, the slugs of the items that block it in `blocked_by`, and its other blockers, as `<component>/<slug>` in the order `clm pending` prints them, in `waits_on`: the requirements its `Needs` name, those a move took an `obsolete` one to, and the rules a capability waits on, in one list (`priority-orders-nodes` seeds through each). It SHALL hold no field `needs_pending`.

#### Scenario: states
- GIVEN component `a` with `a/x` at version `22222222` cited at `11111111`, `a/y` uncited at version `33333333`, `a/z` current, and a marker at `t.rs:9` citing `a/gone`, which an `applied` change removed
- WHEN `clm pending a` runs
- THEN it prints `a/x 22222222 behind 11111111`, `a/y 33333333 unbuilt` and `a/gone obsolete t.rs:9`, and nothing for `a/z`

#### Scenario: stale marker in another component's files
- GIVEN components `a` and `b` with `paths` `["crates/a/**"]` and `["crates/b/**"]`, and `a/x` at version `22222222` cited at `crates/a/t.rs:2` at `22222222` and at `crates/b/t.rs:7` at `11111111`
- WHEN `clm pending` runs
- THEN it prints `a/x 22222222 behind 11111111` for `a`, and nothing for `b`

#### Scenario: entry pending on the root
- GIVEN `capability/c` cited by nothing, and `backlog/q.md` with `blocks: [capability/c]`
- WHEN `clm pending clm` runs
- THEN it prints `capability/c <version> unbuilt blocked backlog/q`

#### Scenario: entries that are never pending
- GIVEN `architecture/a` enforced by `n/x`, which is unbuilt, and `capability/u` holding `#### Untested`
- WHEN `clm pending` runs
- THEN it prints `n/x <version> unbuilt` and no line for `architecture/a` or `capability/u`

#### Scenario: capability node
- GIVEN component `capability` and `capability/c` cited by nothing
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt`, and `clm pending clm` prints no line for it

#### Scenario: removed node
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, a marker at `crates/v/tests/t.rs:4` citing `g/x`, and component `v` with `paths: ["crates/v/**"]`
- WHEN `clm pending v` runs
- THEN it prints `g/x obsolete crates/v/tests/t.rs:4`

#### Scenario: removed node, files of two nodes
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, markers citing `g/x` at `crates/v/t.rs:4` and `crates/w/t.rs:7`, and components `v` and `w` with `paths` `["crates/v/**"]` and `["crates/w/**"]`
- WHEN `clm pending` runs
- THEN it prints `g/x obsolete crates/v/t.rs:4` for `v` and `g/x obsolete crates/w/t.rs:7` for `w`

#### Scenario: removed node, unowned file
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, and a marker at `old/t.rs:4` citing `g/x` that no component's `paths` match
- WHEN `clm pending clm` runs
- THEN it prints `g/x obsolete old/t.rs:4`

#### Scenario: blocked
- GIVEN `a/y` uncited, and `backlog/q.md` with `blocks: [a/y]`
- WHEN `clm pending a` runs
- THEN its line for `a/y` ends with ` blocked backlog/q`

#### Scenario: nothing pending
- GIVEN a component whose requirements are all current
- WHEN `clm pending <component>` runs
- THEN it prints nothing and exits 0

#### Scenario: blocked by a need
- GIVEN `a/y` uncited with `#### Needs` naming `b/z`, `b/z` uncited, and `backlog/q.md` with `blocks: [a/y]`
- WHEN `clm pending a` runs
- THEN its line for `a/y` ends with ` blocked backlog/q,b/z`

#### Scenario: moved requirement waits for the move
- GIVEN components `a` and `a.b`, an `applied` change whose delta for `a` removes `a/x` and whose delta for `a.b` adds `a.b/x`, a marker at `t.rs:4` citing `a/x`, and `a.b/x` uncited
- WHEN `clm pending` runs
- THEN it prints `a/x obsolete t.rs:4 blocked a.b/x` for `a` and `a.b/x <version> unbuilt` for `a.b`

#### Scenario: move landed, marker left behind
- GIVEN the same components and change, a marker at `t.rs:4` citing `a/x`, and `a.b/x` cited at its current version
- WHEN `clm pending a` runs
- THEN it prints `a/x obsolete t.rs:4`, with nothing after it

#### Scenario: need satisfied
- GIVEN `a/y` uncited with `#### Needs` naming `b/z`, and `b/z` cited at its current version
- WHEN `clm pending a` runs
- THEN its line for `a/y` is `a/y <version> unbuilt`, with nothing after it

#### Scenario: capability waits on its serving rules
- GIVEN component `capability`, `capability/c` cited by nothing, component `a` with `serves: [capability/c]` holding `a/x` uncited, `a/y` cited at an older version and `a/z` current, and component `b` holding `b/w` uncited with a `#### Serves` block naming `capability/c`
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt blocked a/x,b/w`

#### Scenario: serving rules built
- GIVEN component `capability`, `capability/c` cited by nothing, and every requirement whose effective serves names it current or cited at an older version
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt`, with nothing after it

#### Scenario: edited capability waits on a new serving rule
- GIVEN component `capability`, `capability/c` cited at an older version, and component `a` with `serves: [capability/c]` holding `a/x` uncited
- WHEN `clm pending capability` runs
- THEN its line for `capability/c` ends with ` blocked a/x`

#### Scenario: serving rule on an optional node
- GIVEN component `capability`, `capability/c` cited by nothing, `backlog/q.md` with `blocks: [capability/c]`, and component `w` with `optional: true` and `serves: [capability/c]` holding `w/x` uncited
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt blocked backlog/q,w/x`, and loading reports no violation

#### Scenario: every wait in one list
- GIVEN `a/y` uncited with `#### Needs` naming `b/z`, `b/z` uncited, component `capability`, `capability/c` cited by nothing, and component `w` with `serves: [capability/c]` holding `w/x` uncited
- WHEN `pending(graph, markers)` runs
- THEN `a/y`'s `waits_on` is `[b/z]` and `capability/c`'s is `[w/x]`, and each `blocked_by` is empty

#### Serves
- capability/build-from-spec
- capability/see-project-state
- architecture/component-reconciliation
- architecture/needs-alone-order

#### Needs
- model/human-layers
- model/reserved-ids

### Requirement: status-report
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds. For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. A section with no rows has the single row `none`. It SHALL exit 0, or print the violations and exit 1 when the spec graph has violations.

#### Scenario: entries on the root's row
- GIVEN root `clm` with no requirements, `architecture/a` enforced by a current rule of another component, `capability/b` cited by nothing, `capability/c` cited at its current version, and `capability/u` holding `#### Untested`
- WHEN `clm status` runs
- THEN the `clm` row reads `1/2`

#### Scenario: one pending change
- GIVEN component `a` (active, requirements `a/x` current and `a/y` not), and change `20260101-c` with status `applied` whose delta adds `a/x` and `a/y`
- WHEN `clm status` runs
- THEN its `components` section has a row with the cells `a`, `active` and `1/2`, and its `changes` section has a row with the cells `20260101-c` and `1/2 requirements` in the `applied` group

#### Scenario: complete change
- GIVEN change `20260101-c` with status `applied` whose two added requirements are both current
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `2/2 requirements` and `complete`

#### Scenario: requirement removed later
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, `a/x` current, and no component holding `a/y`
- WHEN `clm status` runs
- THEN the `changes` section has a row with the cells `20260101-c`, `1/1 requirements` and `complete`

#### Scenario: node renamed later, not yet built
- GIVEN an `applied` change `20260101-c` whose delta for `g` adds `g/x`, no component `g`, an `applied` change `20260102-r` whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements` and whose `create: true` delta for `v` adds `v/x`, and `v/x` uncited
- WHEN `clm status` runs
- THEN the row for `20260101-c` has the cells `20260101-c` and `0/1 requirements`, and no `complete` cell

#### Scenario: node renamed later, built
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

### Requirement: change-progress
`change_progress(graph, markers)` SHALL return, for each change outside the archive, in ID order, its ID; `current` and `total`, its requirements current out of the requirements its deltas add or modify; `obsolete`, the markers (`<file>:<line>`) citing a requirement its deltas remove that the spec no longer holds; and whether it is `complete`, all counted as `check.coverage/status-report` counts them. For every change, the row `clm status` prints SHALL carry the same `<current>/<total>` and the same `complete`, and `complete_changes` SHALL name exactly the changes it marks complete.

#### Scenario: same counts as status
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, `a/x` current and `a/y` cited at an older version, and an `applied` change `20260101-d` whose two added requirements are both current
- WHEN `change_progress` runs on the graph and markers that `clm status` reads
- THEN it gives `20260101-c` 1 of 2 current and not complete and `20260101-d` 2 of 2 and complete, `clm status` prints `1/2 requirements` and `2/2 requirements` with `complete` only on `20260101-d`, and `complete_changes` returns `20260101-d` alone

#### Scenario: removed requirement still cited
- GIVEN an `applied` change `20260101-c` whose delta removes `a/x`, no component holding `a/x`, and a test citing `a/x` at `t.rs:3`
- WHEN `change_progress` runs
- THEN `20260101-c` has `obsolete` `t.rs:3` and is not complete

#### Serves
- capability/see-project-state
- capability/wrap-up-session
- architecture/component-reconciliation

### Requirement: coverage-builds-for-wasm
The `clm-coverage` crate SHALL build for `wasm32-unknown-unknown` with its default features disabled.

#### Scenario: wasm build
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `cargo build -p clm-coverage --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser
- architecture/web-optional

### Requirement: owner
`owner(graph, path)` SHALL return the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `paths` do, matching each glob with `matches`. `pending` SHALL decide a file's component with this `owner`, falling back to the root when it returns none, as it does now.

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Serves
- capability/build-from-spec
- architecture/path-ownership

### Requirement: state-names
`check.coverage` SHALL expose, as a public constant re-exported through `clm-check`, every requirement `State` with the word `clm coverage` and `clm pending` print for it (`current`, `behind`, `unbuilt`, `obsolete`). A test SHALL fail when `State` gains a variant the list leaves out.

#### Scenario: list matches the type
- GIVEN the crate
- WHEN a test reads the list
- THEN every variant of `State` appears once, with the word the reports print

#### Serves
- architecture/plain-terms

### Requirement: marker-spans
`clm-coverage` SHALL offer `marker_spans(text)`, returning each marker `markers_in` finds in `text`, in the same order and with the same ID and version, with the byte range of its whole text (`req:` through its slug, and through its version when it carries one), so a caller that rewrites markers in place (`rename/marker-rewrite`) rewrites exactly the markers the coverage counts. It SHALL build without the `checkout` feature.

#### Scenario: two markers on a line
- GIVEN the text `// req:a/x@11111111 and req:b/y`
- WHEN `marker_spans` runs
- THEN it returns `a/x` at version `11111111` spanning bytes `3..19`, and `b/y` with no version spanning bytes `24..31`, each range end-exclusive

#### Scenario: the same markers as markers_in
- GIVEN a file's text with markers, a removal marker and a `req:` that follows a word character
- WHEN `marker_spans` and `markers_in` run on it
- THEN they find the same markers in the same order
