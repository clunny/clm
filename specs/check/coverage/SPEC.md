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
- `tests_in(path, text)`: the tests and named groups of tests in one file, each with its name and its lines, and `markers_outside_tests(path, text)`: the `req:` markers on none of those lines (`test-spans`).
- `markers_in(path, text)`: the `req:` markers in one file that count, those inside a test (`markers-in-tests`), none for Markdown; `scan(root)`, with the default feature `checkout`: the `req:` and `clm:remove-by` markers of every file a checkout lists, as `Scan { req, remove_by }`.
- `report(graph, markers)`: each requirement's `Coverage`, human-layer entries included (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements (`pending-report`).
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)` and `complete_changes`, built on `change_progress` (`status-report`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` glob matches a repository-relative path, as `pending` and `check`'s `paths-disjoint` read `paths`.
- `obsolete_tests(graph, files)` and `deleted_tests(base_files, head_files)`: the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`); `check.affected` reads it through `clm-check` (`check/path-ownership`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust, TypeScript and JavaScript) or indentation (Python) outside line comments. That is approximate on purpose: a brace inside a string can end a span early or late, which costs one marker its standing and shows in `clm coverage`'s `uncounted` line, never a crash. In a file of any other extension CLM can't tell a test, so every marker counts as before; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## Requirements

### Requirement: coverage-report
`clm coverage` SHALL print one line per requirement, and per human-layer entry (`architecture/<slug>`, `capability/<slug>`, `model/human-layers`), all sorted together by ID: `<id> current <file>:<line>[ <file>:<line>...]` when markers cite it, every one cites its current version and no backlog item's `unproven` names it at that version (`model/parse-backlog`), `<id> behind <file>:<line>[ ...]` listing the markers that cite it at another version or without one when any does, even beside markers at its current version, which the line leaves out (`stale-markers-keep-behind`), otherwise `<id> unbuilt`, which a requirement that an item's `unproven` names at its current version reads whatever cites it, since the mutation pass found that its tests notice no change to its code (`verify/mutation-pass`). An architecture entry is never cited (`check/dangling-markers`): its line is `<id> current` when it holds an `#### Enforced by` block and every rule it names is current, `<id> pending <rule>[,<rule>...]` naming, in block order, the rules it names that aren't, `<id> untested` when it holds only an `#### Untested` block, and `<id> unenforced` when it holds neither, which `check/unenforced` fails; a capability holding `#### Untested` is `<id> untested` (`human-layer-proof`); and, after those, one line `<id> obsolete <file>:<line>[ <file>:<line>...]` per requirement that no longer exists but that a change outside the archive removed and a marker still cites. It SHALL exit 0.

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

#### Scenario: named unproven
- GIVEN `a/x` at version `aaaaaaaa` cited at `t.rs:2` at that version, and `backlog/u.md`, a `bug` item with `unproven: [a/x@aaaaaaaa]`
- WHEN `clm coverage` runs, and again once the item is deleted
- THEN the first prints `a/x unbuilt` and the second `a/x current t.rs:2`

#### Scenario: unproven at an older version
- GIVEN the same item, and `a/x` reworded to version `bbbbbbbb` and cited at it
- WHEN `clm coverage` runs
- THEN it prints `a/x current t.rs:2`

#### Needs
- model/parse-backlog
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
`clm pending [<component>]` SHALL print one line per pending requirement of every component, or of `<component>`, sorted by component ID then requirement ID: `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` with the versions its markers cite other than its current one (`none` for a marker without one), sorted, or `<id> obsolete <file>:<line>[ ...]`; each followed by ` blocked <blocker>[,<blocker>...]` when it is blocked, each blocker `backlog/<slug>` for a backlog item that names it in `blocks` or `<component>/<slug>` for a pending requirement its `Needs` name (`requirement-needs`), for an `obsolete` one a pending requirement a move took it to, or for a capability a requirement serving it that holds it back, items first, each group sorted. An `obsolete` requirement SHALL be blocked by each pending requirement that the moves `status-report` defines take it to, followed as `progress-follows-moves` follows them to the IDs the spec holds, on whatever component each is (`moved-markers-wait`): the build of that requirement writes its own tests in its own component's files, so the obsolete line waits for it, and the obsolete requirement's own component then removes its markers, tests and code (`coupled-changes-sequence`). A capability that is pending SHALL count as pending on the component that builds the capabilities' tests, which `pending` takes from the model's `Graph::capability_tests_node` (`model/capability-tests-component`) rather than deciding itself, and is blocked like any requirement by the items whose `blocks` name it. A pending capability, `unbuilt` or `behind`, SHALL also be blocked by each requirement whose effective serves (`model/serves`) names it, on whatever component, optional or not, that is `unbuilt`, or `behind` and itself blocked, by an item whose `blocks` names it or by any requirement in its own `waits_on`, whatever that requirement's state and whatever blocks it in turn; and by no current one, nor any `behind` one that nothing blocks (`serving-rules-order-capabilities`). So its test is built after the rules it exercises, and isn't offered while a changed rule it exercises waits on other work. A capability holding `#### Untested` is never pending, and neither is an architecture entry: the rules its `#### Enforced by` names are pending on their own components (`human-layer-proof`). It SHALL exit 0, printing nothing when nothing is pending, or print the violations and exit 1 when the spec graph has violations. An `obsolete` marker whose ID names a component the spec graph doesn't hold SHALL count as pending on the component `owner` gives for the marker's file, or on the root `clm` when it gives none, however the component went: removed by a delta with `remove: true` (`node-removal`) or gone without one. A removed requirement cited in files of several components is pending on each of them, each line listing only that component's files. `pending(graph, markers)` SHALL return the same for a graph and markers loaded without a checkout, with each requirement's priority, the slugs of the items that block it in `blocked_by`, and its other blockers, as `<component>/<slug>` in the order `clm pending` prints them, in `waits_on`: the requirements its `Needs` name, those a move took an `obsolete` one to, and the rules a capability waits on, in one list (`priority-orders-nodes` seeds through each).

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

#### Scenario: component gone without a removal delta
- GIVEN no component `g`, an `applied` change whose delta for `g` lists `g/x` under `REMOVED Requirements` without `remove: true`, a marker at `crates/v/t.rs:1` citing `g/x`, and component `v` with `paths` `["crates/v/**"]`
- WHEN `clm pending v` runs
- THEN it prints `g/x obsolete crates/v/t.rs:1`

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

#### Needs
- model/human-layers
- model/reserved-ids

### Requirement: status-report
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

### Requirement: change-progress
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
`owner(graph, path)` SHALL return the component whose `tests` match `path` (`model/test-ownership-keys`, `components-own-their-tests`), or, when none's do, the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `tests` or `paths` do, matching each glob with `matches`; when two components' `tests` match, it SHALL return the first by ID, which `check/paths-disjoint` reports. `pending` SHALL decide a file's component with this `owner`, falling back to the root when it returns none.

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Scenario: tests inside another component's paths
- GIVEN component `harness` with `paths: ["tests/**"]` and component `a` with `tests: ["tests/a/**"]`
- WHEN `owner` runs on `tests/a/conftest.py` and `tests/helpers.py`
- THEN it returns `a` and `harness`

#### Serves
- capability/build-from-spec
- architecture/path-ownership

#### Needs
- model/test-ownership-keys

### Requirement: state-names
`check.coverage` SHALL expose, as a public constant re-exported through `clm-check`, every requirement `State` with the word `clm coverage` and `clm pending` print for it (`current`, `behind`, `unbuilt`, `obsolete`). A test SHALL fail when `State` gains a variant the list leaves out.

#### Scenario: list matches the type
- GIVEN the crate
- WHEN a test reads the list
- THEN every variant of `State` appears once, with the word the reports print

#### Serves
- architecture/plain-terms

### Requirement: marker-spans
`clm-coverage` SHALL offer `marker_spans(text)`, returning each `req:` marker in `text`, inside a test or not, in the order they appear, with the ID and version `markers_in` and `markers_outside_tests` give it, with the byte range of its whole text (`req:` through its slug, and through its version when it carries one), so a caller that rewrites markers in place (`rename/marker-rewrite`) rewrites every marker, the ones the coverage counts and the ones it reports as `uncounted`. It SHALL build without the `checkout` feature.

#### Scenario: two markers on a line
- GIVEN the text `// req:a/x@11111111 and req:b/y`
- WHEN `marker_spans` runs
- THEN it returns `a/x` at version `11111111` spanning bytes `3..19`, and `b/y` with no version spanning bytes `24..31`, each range end-exclusive

#### Scenario: every marker of the file
- GIVEN the text of `t.rs` with a marker in a test, a marker in a function that is no test, a removal marker and a `req:` that follows a word character
- WHEN `marker_spans`, `markers_in` and `markers_outside_tests` run on it
- THEN `marker_spans` finds the two markers in order, the first the one `markers_in` returns and the second the one `markers_outside_tests` returns

### Requirement: removed-capabilities
A capability SHALL count as *removed* while a change outside the archive, whatever its status, lists its slug under `removes_capabilities` (`model/parse-changes`) and `CAPABILITIES.md` holds no entry of that slug: `removed(graph)` SHALL give `capability/<slug>` with the IDs of those changes, sorted, as it gives a requirement a delta removed, and SHALL NOT give a listed capability that `CAPABILITIES.md` holds, which is a capability like any other. A marker citing a removed capability SHALL read as `obsolete`, as one citing a removed requirement does: `clm coverage` prints `capability/<slug> obsolete <file>:<line>[ ...]` among its obsolete lines (`coverage-report`), and `clm pending` prints the same line, with every marker's location, where `pending-report` places an obsolete requirement, which is the component `capability` when the graph holds it, and otherwise the owner of each marker's file, or the root. Nothing blocks the line, since no item, need, move or serving rule can name a capability that is gone, and its priority is the highest `priority` among the changes that list it, as a requirement a delta removes takes its change's. So a spec branch that removes a cited capability by hand passes `clm check` (`check/dangling-markers`), and the build of the component the line is pending on removes the tests (`removed-capabilities-build-out`). Once the change is archived, the capability is no longer removed and its markers dangle, so the change can't be complete while one is left (`status-report`).

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

### Requirement: test-spans
`clm-coverage` SHALL offer `tests_in(path, text)`, returning the tests and the named groups of tests in one file's text, in the order they start, each with its name, whether it is a group, and its first and last line, the first being the first of the comment and attribute lines directly above it with no blank or code line between, and SHALL find them by the file's extension:
- `.rs`: a test is a function that an attribute whose path ends in `test`, `rstest` or `test_case` precedes (`#[test]`, `#[tokio::test]`), named by the function, from those lines to the line that closes its body; there are no groups.
- `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs`: a test is a call of `it` or `test`, and a group a call of `describe` or `test.describe`, each with or without the modifiers `check/skipped-tests` reads and `.only`, `.each`, `.concurrent` and `.serial`, starting a line after leading whitespace, named by its first argument when that is a string literal and by an empty name otherwise, from those lines to the line that closes the call.
- `.py`: a test is a function whose name starts `test`, and a group a class whose name starts `Test`, named by it, from the decorator and comment lines directly above it to the last line indented deeper than its `def` or `class` line.
- any other extension: none.

It SHALL offer `markers_outside_tests(path, text)`, returning each marker `marker_spans` finds in `text` on a line that belongs to no test or group, as `Marker`s, and nothing for a Markdown file or a file of another extension than those above. Both SHALL build without the `checkout` feature. Neither changes what `markers_in` or `scan` returns.

#### Scenario: a Rust test and a helper
- GIVEN the text of `t.rs`: a comment line holding `req:a/x`, then `#[test]`, then `fn proves_x() {`, a line holding `req:a/y`, `}`, a blank line, and `fn helper() {` with a line holding `req:a/z` and `}`
- WHEN `tests_in` and `markers_outside_tests` run on it
- THEN `tests_in` returns one test, `proves_x`, from line 1 to line 5, and `markers_outside_tests` returns `a/z` alone

#### Scenario: a group above its tests
- GIVEN the text of `a.test.ts`: line 1 a comment holding `req:a/h`, a blank line, a comment holding `req:a/g`, then `describe('group', () => {`, `  it('works', () => {`, a line holding `req:a/x`, `  });` and `});`
- WHEN both run on it
- THEN `tests_in` returns the group `group` from line 3 to line 8 and the test `works` inside it, and `markers_outside_tests` returns `a/h` alone

#### Scenario: a Playwright group
- GIVEN the text of `e.spec.ts`: a comment holding `req:a/x` directly above `test.describe('phone', () => {` and its closing line
- WHEN `markers_outside_tests` runs on it
- THEN it returns nothing

#### Scenario: a Python test
- GIVEN the text of `test_a.py`: `def test_x():` with an indented line holding `req:a/x`, then a line at the left margin holding `req:a/y`
- WHEN `markers_outside_tests` runs on it
- THEN it returns `a/y` alone

#### Scenario: another language
- GIVEN the text of `a_test.go` holding `req:a/x` in a comment
- WHEN `tests_in` and `markers_outside_tests` run on it
- THEN each returns nothing

### Requirement: markers-in-tests
`markers_in(path, text)` and `scan(root)`'s `req` SHALL hold, for a file of an extension `test-spans` reads, only the markers on a line of a test or a group of tests, and for any other file every marker, none for Markdown, so a marker in code that isn't a test proves nothing (`markers-count-in-tests`). After the obsolete lines `clm coverage` SHALL print one line `<id> uncounted <file>:<line>[ <file>:<line>...]` for each requirement or capability that exists and that a marker outside a test cites, sorted by ID, so a build finds the markers to move. The capability test `crates/clm/tests/capability/prove_by_tests.rs` writes its marker inside a test function; this requirement's build adds to it that the same marker in a function with no test attribute leaves the requirement `unbuilt` and prints the `uncounted` line.

#### Scenario: a marker in code that is no test
- GIVEN `a/x`, and `src/lib.rs` holding `req:a/x` at its current version in the body of a function with no test attribute, cited nowhere else
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt`, and after it `a/x uncounted src/lib.rs:<line>`

#### Scenario: a marker in a test
- GIVEN the same marker in the body of a `#[test]` function in `tests/x.rs`
- WHEN `clm coverage` runs
- THEN it prints `a/x current tests/x.rs:<line>` and no `uncounted` line

#### Scenario: a file-header marker
- GIVEN `a.test.ts` whose first line is a comment holding `req:a/x`, followed by a blank line and tests
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt` and `a/x uncounted a.test.ts:1`

#### Scenario: a language CLM doesn't read
- GIVEN `a_test.go` holding `req:a/x` at its current version in a comment
- WHEN `clm coverage` runs
- THEN it prints `a/x current a_test.go:<line>`

#### Scenario: same as the files
- GIVEN a checkout with markers inside and outside tests
- WHEN coverage is computed with `markers_in` and `report` from the files' paths and texts, and with `clm coverage`
- THEN both report the same states at the same locations

#### Needs
- check/dangling-markers

### Requirement: test-lists
`clm coverage --obsolete-tests` SHALL print, in place of the coverage, one line for each test (`test-spans`, groups left out) that holds a marker citing a requirement or capability a change outside the archive removed (`removed`), sorted by file and then by line: `DELETE <file>::<name>` followed by a tab and `cites only removed: <id>[, <id>...]` when every marker on the test's lines cites such a requirement, and otherwise `UNCITE <file>::<name>` followed by a tab and `loses: <id>[, <id>...]`, the IDs sorted, and before them the line `# tests to delete: <n>; tests that only lose a marker: <m>`. `clm coverage --deleted-tests <base>` SHALL print one line `DELETED <file>::<name>`, a tab and the IDs it cited, for each test that holds a marker in the tree at the revision `<base>` and that the working tree's file of that path, or its absence, holds no test of that name for, sorted the same way, and before them `# tests deleted since <base>: <n>`. Each SHALL exit 0, and exit 1 naming the revision when `<base>` resolves to no commit. The library calls behind them, `obsolete_tests` and `deleted_tests`, SHALL take the files' paths and texts, so they build without the `checkout` feature.

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `only_x` citing `a/x` alone and a test `both` citing `a/x` and `a/y`, both in `tests/t.rs`
- WHEN `clm coverage --obsolete-tests` runs
- THEN it prints `# tests to delete: 1; tests that only lose a marker: 1`, then `UNCITE tests/t.rs::both` with `loses: a/x` and `DELETE tests/t.rs::only_x` with `cites only removed: a/x`, in line order

#### Scenario: a branch that deletes a test
- GIVEN a commit `b` whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and a working tree whose `tests/t.rs` holds only `one`
- WHEN `clm coverage --deleted-tests b` runs
- THEN it prints `# tests deleted since b: 1` and `DELETED tests/t.rs::two` with `a/x`

#### Scenario: a renamed test
- GIVEN the same, with `two` renamed `second` in the working tree
- WHEN it runs
- THEN it lists `tests/t.rs::two` as deleted, since a program can't tell a rename from a removal

#### Scenario: an unknown base
- GIVEN no commit `nope`
- WHEN `clm coverage --deleted-tests nope` runs
- THEN it exits 1 naming `nope`
