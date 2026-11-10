---
component: check.coverage
base: { design: "4d75a51e9584ade4" }
---

## MODIFIED Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

## MODIFIED Requirements
### Requirement: pending-report <!-- base: 89a3068a8f10b8da -->
`clm pending [<component>]` SHALL print one line per pending requirement of every component, or of `<component>`, sorted by component ID then requirement ID: `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` with the versions its markers cite other than its current one (`none` for a marker without one), sorted, or `<id> obsolete <file>:<line>[ ...]`; each followed by ` blocked <blocker>[,<blocker>...]` when it is blocked, each blocker `backlog/<slug>` for a backlog item that names it in `blocks` or `<component>/<slug>` for a pending requirement its `Needs` name (`requirement-needs`), for an `obsolete` one a pending requirement a move took it to, or for a capability a requirement serving it that holds it back, items first, each group sorted. An `obsolete` requirement SHALL be blocked by each pending requirement that the moves `status-report` defines take it to, followed as `progress-follows-moves` follows them to the IDs the spec holds, on whatever component each is (`moved-markers-wait`): the build of that requirement re-points the markers, so the obsolete line waits for it. A capability that is pending SHALL count as pending on the component that builds the capabilities' tests, which `pending` takes from the model's `Graph::capability_tests_node` (`model/capability-tests-component`) rather than deciding itself, and is blocked like any requirement by the items whose `blocks` name it. A pending capability, `unbuilt` or `behind`, SHALL also be blocked by each requirement whose effective serves (`model/serves`) names it, on whatever component, optional or not, that is `unbuilt`, or `behind` and itself blocked, by an item whose `blocks` names it or by any requirement in its own `waits_on`, whatever that requirement's state and whatever blocks it in turn; and by no current one, nor any `behind` one that nothing blocks (`serving-rules-order-capabilities`). So its test is built after the rules it exercises, and isn't offered while a changed rule it exercises waits on other work. A capability holding `#### Untested` is never pending, and neither is an architecture entry: the rules its `#### Enforced by` names are pending on their own components (`human-layer-proof`). It SHALL exit 0, printing nothing when nothing is pending, or print the violations and exit 1 when the spec graph has violations. An `obsolete` marker whose ID names a component that no longer exists, since a change removed it (`node-removal`), SHALL count as pending on the component `owner` gives for the marker's file, or on the root `clm` when it gives none. A removed requirement cited in files of several components is pending on each of them, each line listing only that component's files. `pending(graph, markers)` SHALL return the same for a graph and markers loaded without a checkout, with each requirement's priority, the slugs of the items that block it in `blocked_by`, and its other blockers, as `<component>/<slug>` in the order `clm pending` prints them, in `waits_on`: the requirements its `Needs` name, those a move took an `obsolete` one to, and the rules a capability waits on, in one list (`priority-orders-nodes` seeds through each).

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

#### Scenario: capability component
- GIVEN component `capability` and `capability/c` cited by nothing
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt`, and `clm pending clm` prints no line for it

#### Scenario: removed component, files of two components
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, markers citing `g/x` at `crates/v/t.rs:4` and `crates/w/t.rs:7`, and components `v` and `w` with `paths` `["crates/v/**"]` and `["crates/w/**"]`
- WHEN `clm pending` runs
- THEN it prints `g/x obsolete crates/v/t.rs:4` for `v` and `g/x obsolete crates/w/t.rs:7` for `w`

#### Scenario: removed component, unowned file
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, and a marker at `old/t.rs:4` citing `g/x` that no component's `paths` match
- WHEN `clm pending clm` runs
- THEN it prints `g/x obsolete old/t.rs:4`

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
- GIVEN component `capability`, `capability/c` cited by nothing, and every requirement whose effective serves names it current, or cited at an older version with no item naming it and no `#### Needs`
- WHEN `clm pending capability` runs
- THEN it prints `capability/c <version> unbuilt`, with nothing after it

#### Scenario: edited capability waits on a new serving rule
- GIVEN component `capability`, `capability/c` cited at an older version, and component `a` with `serves: [capability/c]` holding `a/x` uncited
- WHEN `clm pending capability` runs
- THEN its line for `capability/c` ends with ` blocked a/x`

#### Scenario: changed serving rule that waits
- GIVEN component `capability`, `capability/c` cited at an older version, component `a` with `serves: [capability/c]` holding `a/x` cited at an older version with `#### Needs` naming `b/z`, `a/w` cited at an older version, and `a/y` cited at an older version with nothing blocking it, `backlog/q.md` with `blocks: [a/w]`, and `b/z` uncited
- WHEN `clm pending` runs
- THEN its line for `capability/c` ends with ` blocked a/w,a/x`, its line for `a/x` with ` blocked b/z` and its line for `a/w` with ` blocked backlog/q`

#### Scenario: changed serving rule released
- GIVEN the components and item of "changed serving rule that waits", with `b/z` cited at its current version and `backlog/q.md` deleted
- WHEN `clm pending capability` runs
- THEN its line for `capability/c` is `capability/c <version> behind <cited>`, with nothing after it

#### Scenario: serving rule on an optional component
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

### Requirement: status-report <!-- base: 55590ca528961920 -->
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds. For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. A section with no rows has the single row `none`. It SHALL exit 0, or print the violations and exit 1 when the spec graph has violations.

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

### Requirement: owner <!-- base: 5180c8dab3447ee8 -->
`owner(graph, path)` SHALL return the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `paths` do, matching each glob with `matches`. `pending` SHALL decide a file's component with this `owner`, falling back to the root when it returns none.

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Serves
- capability/build-from-spec
- architecture/path-ownership
