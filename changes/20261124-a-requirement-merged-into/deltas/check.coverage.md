---
component: check.coverage
base: { design: "b6e09abf0b7a0df8" }
---

## MODIFIED Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status`, `lists`, `spans` and `glob`. Its readers import it directly (`check-coverage-split`): `clm coverage` and `clm pending` reach it through `clm-check`, and `clm status` through `clm-view`. A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers: `clm:remove-by ` not preceded by an ASCII letter, digit or `_`, followed by a change ID of eight ASCII digits, `-` and one or more of `a-z`, `0-9` and `-`, the marker `check.unreferenced/removal-marked` accepts, so every item that detector skips for a marker has its marker checked by `check/remove-by-markers` (`removal-markers`). A `req:` marker's word character is any Unicode letter or digit, or `_`; the removal marker's is ASCII alone, as `clm unreferenced`, which depends on `vcs` alone, reads it. Where `git ls-files` fails, as outside a git work tree such as a tree `clm wrap-up` extracts, every file there is tracked, so `scan` walks the files under the root instead, leaving out only a `.git` directory and `target`, as `rename` reads a tree with no `.git`, and reading a directory entry's own type, so a link to a directory is never followed and a cycle of links ends (`markers-from-files`). One call, `read_files`, gives those files and their texts, and `scan` reads them through it; `clm-check` reads the files for `clm coverage --obsolete-tests` and `--deleted-tests` through it too, so the two never list different files (`check/coverage-lists`, `a-wait-names-a-line`). `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. A line is one requirement on one component, and an obsolete requirement may have several, so an ID alone names no line: `waits` resolves an entry of `waits_on` to lines, for the queue's priorities, the blockers view and the build prompt alike (`wait-lines`, `a-wait-names-a-line`). An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Status** (`status-report`). `status_report` gives `clm status`'s rows as cells and carries no text form of its own: `view/status-report` lays `clm status` out, the `none` of an empty section included, so the report has no `Display` and nothing here writes the text.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. A move is made in one of two ways. A removal and an addition of the same slug on another component is one, as a rename is: only `ADDED Requirements` make it, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A merge into a requirement of another slug, on the same component or another, is one only where the change records it in `moves` (`model/move-records`), since no program can tell from the deltas which requirement took a removed one's sentences; the model has checked that the change removes the one and adds or modifies the other, so the map takes each entry as written. The two kinds go into the one map and are followed alike. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `a-marker-counts-for-its-own-test`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside comments and string and character literals, since a test that names a brace, as `find('{')` or a JSON string does, or a URL whose `//` would read as a comment, is common and was misread. The scanner reads only what it must to skip a literal or a comment: a block comment, `/* … */`, running across lines and nesting in Rust, since a quote in one, as `/* a 12" ruler */`, would otherwise open a string that never closes; Rust's `"…"` and raw `r#"…"#` strings, with any `b` or `c` prefix and running across lines, and a character literal, `'{'` or `'\''`, told from a lifetime or label (`'a`) by its closing quote; and the three quotes of TypeScript and JavaScript, a template literal's `${…}` read as part of the string and a `'` or `"` string ending at its line's end, so an apostrophe in a regular expression or JSX text costs one line, not the file. A Rust file is read for literals and block comments from its first line, so a fixture string or a commented-out test holding `#[test]` neither starts a test nor bounds the one holding it. Rust's literals and comments are then read whole, but what TypeScript and JavaScript don't read, a regular expression literal holding a quote or a parenthesis, or a template literal nested in a `${…}`, and a brace a line scanner still miscounts in either, can leave a span open, so a test's span also ends no later than the line above the first line of the next test (or, in TypeScript and JavaScript, group): a misread costs that one test's markers, which show in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never the tests after it and never a crash. The bound matters beyond the count: the mutation pass judges each marker by the innermost test holding it (`verify/mutation-pass`), so a span that ran to the end of its file had it run one test for every later marker and file their requirements unproven, though other tests proved them. A group takes no such bound, since its call holds its tests: a misread in a group's own code, outside its tests (a hook matching ``/`/``), can still run the group's span to the end of its file, which leaves its tests' spans and the innermost test of every marker as they are and costs nothing, since a marker on a group's own lines counts for no test; reading regular expressions to close that gap would mean telling one from a division and from JSX's `</`, much code for a rare case. Python's spans end by indentation and read no literal. A group is read only to bound the tests in it: a marker on its own lines, the comment above its call included, counts for none of them (`a-marker-counts-for-its-own-test`), since a group's tests prove it severally, and a test inside one could then be deleted unseen (`land/deleted-tests-approved`) while `clm health` called it uncited (`check.health/rule-signals`); `deleted_tests` and `uncited_tests` read a test's own lines, and the coverage reads the same ones. In a file of any other extension CLM can't tell a test, so every marker in it counts; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## MODIFIED Requirements

### Requirement: status-report <!-- base: 8aad5594a107457d -->
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status as `active`, `unbuilt` or `deprecated`, and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds, nor a capability it lists under `removes_capabilities` that `CAPABILITIES.md` no longer holds (`removed-capabilities`). For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`), and moves `<a>/<x>` to `<b>/<y>`, `<b>` being `<a>` or another component, when its `moves` holds an entry with `requirement: <a>/<x>` and `to: <b>/<y>`, as a merge into a requirement of another slug does (`model/move-records`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. The command's exit status is `view.repo/arguments-and-exit`'s.

#### Scenario: entries on the root's row
- GIVEN root `clm` with no requirements, `architecture/a` enforced by a current rule of another component, `capability/b` cited by nothing, `capability/c` cited at its current version, and `capability/u` holding `#### Untested`
- WHEN `clm status` runs
- THEN the `clm` row reads `1/2`

#### Scenario: one pending change
- GIVEN component `a` (active, requirements `a/x` current and `a/y` not), and change `20260101-c` with status `applied` whose delta adds `a/x` and `a/y`
- WHEN `clm status` runs
- THEN its `components` section has a row with the cells `a`, `active` and `1/2`, and its `changes` section has a row with the cells `20260101-c` and `1/2 requirements` in the `applied` group

#### Scenario: status words
- GIVEN component `a` with status `active` and component `b` with status `unbuilt`, each with one uncited requirement
- WHEN `clm status` runs
- THEN its `components` section has a row with the cells `a`, `active` and `0/1`, and one with the cells `b`, `unbuilt` and `0/1`

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

#### Scenario: merged later
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x`, no component holding `a/x`, and an `applied` change `20260102-m` whose delta for `a` removes `a/x` and modifies `a/y` and whose `moves` holds `requirement: a/x` with `to: a/y`
- WHEN `clm status` runs, with `a/y` cited at an older version, and again with `a/y` cited at its current version
- THEN the row for `20260101-c` first has the cells `20260101-c` and `0/1 requirements` and no `complete` cell, and then the cells `20260101-c`, `1/1 requirements` and `complete`

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
- replica/snapshot-matches-clm
- model/move-records

### Requirement: pending-report <!-- base: 8d05b4467025688d -->
`clm pending [<component>]` SHALL print one line per pending requirement of every component, or of `<component>`, sorted by component ID then requirement ID: `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` with the versions its markers cite other than its current one (`none` for a marker without one), sorted, or `<id> obsolete <file>:<line>[ ...]`; each followed by ` blocked <blocker>[,<blocker>...]` when it is blocked, each blocker `backlog/<slug>` for a backlog item that names it in `blocks`, on each of its lines when it is `obsolete` (`model/parse-backlog`) or `<component>/<slug>` for a pending requirement its `Needs` name (`requirement-needs`), for an `obsolete` one a pending requirement a move took it to, or for a capability a requirement serving it that holds it back, items first, each group sorted. An `obsolete` requirement SHALL be blocked by each pending requirement that the moves `status-report` defines take it to, followed as `progress-follows-moves` follows them to the IDs the spec holds, on whatever component each is (`moved-markers-wait`): the build of that requirement writes its own tests in its own component's files, so the obsolete line waits for it, and the components the obsolete requirement is pending on then remove its markers and tests, and its own component its code (`coupled-changes-sequence`). A capability that is pending SHALL count as pending on the component that builds the capabilities' tests, which `pending` takes from the model (`model/capability-tests-component`) rather than deciding itself, and is blocked like any requirement by the items whose `blocks` name it. A pending capability, `unbuilt` or `behind`, SHALL also be blocked by each requirement whose effective serves (`model/serves`) names it, on whatever component, optional or not, that is `unbuilt`, or `behind` and itself blocked, by an item whose `blocks` names it or by any requirement in its own `waits_on`, whatever that requirement's state and whatever blocks it in turn; and by no current one, nor any `behind` one that nothing blocks (`serving-rules-order-capabilities`). So its test is built after the rules it exercises, and isn't offered while a changed rule it exercises waits on other work. A capability holding `#### Untested` is never pending, and neither is an architecture entry: the rules its `#### Enforced by` names are pending on their own components (`human-layer-proof`). It SHALL print nothing when nothing is pending. The command's exit status is `view.repo/arguments-and-exit`'s, and narrowing the list to `<component>`, with the refusal of one the spec graph doesn't hold, is `check/pending-narrows-on-a-graph`'s. An `obsolete` marker whose ID names a component the spec graph doesn't hold SHALL count as pending on the component `owner` gives for the marker's file, or on the root `clm` when it gives none, however the component went: removed by a delta with `remove: true` (`node-removal`) or gone without one. A removed requirement cited in files of several components is pending on each of them, each line listing only that component's files. Once the root sets `wiring` (`model/wiring-globs`), an `obsolete` requirement whose component the spec graph holds SHALL also be pending on each other component that owns a file citing it, that component's line listing only its files, while a file a `wiring` glob matches and a file `owner` gives no component for count as the requirement's own component's. Its line on its own component, which lists those files and that component's, SHALL then be blocked by its own ID, standing for its lines on the other components, while any of them is left, `pending` returning that ID in the line's `waits_on` and in no other line's (which lines an entry of `waits_on` stands for is `wait-lines`'s), so the build that deletes a test, which breaks nothing, comes before the build that removes the code the test ran; when its own component has no such file it has no line. A blocker that `moved-markers-wait` gives an obsolete line SHALL be left out of a line pending on that blocker's own component when nothing blocks that blocker, its own `blocked_by` and `waits_on` both empty, since that component's one build then makes it current; a blocker that is itself blocked SHALL stay on that line too, so the build re-cites no test to a requirement it was told to leave out. A removed capability is placed as `removed-capabilities` places it. While the root sets no `wiring`, an obsolete requirement whose component the graph holds SHALL stay pending on that component alone, every file listed. `pending(graph, markers)` SHALL return the same for a graph and markers loaded without a checkout, with each requirement's priority on each of its lines, the highest `priority` among the changes outside the archive whose deltas add, modify or remove it, or `P5` when none does (a removed capability's is `removed-capabilities`'), the slugs of the items that block it in `blocked_by`, and its other blockers, as `<component>/<slug>` in the order `clm pending` prints them, in `waits_on`: the requirements its `Needs` name, those a move took an `obsolete` one to, and the rules a capability waits on, in one list (`priority-orders-nodes` seeds through each).

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

#### Scenario: an obsolete marker in another component's file
- GIVEN a root with `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, an `applied` change that removed `b/gone`, and markers citing `b/gone` at `crates/b/t.rs:3` and `crates/c/t.rs:8`
- WHEN `clm pending` runs, and again once the marker in `crates/c/t.rs` is gone
- THEN the first prints `b/gone obsolete crates/b/t.rs:3 blocked b/gone` for `b` and `b/gone obsolete crates/c/t.rs:8` for `c`, and the second prints `b/gone obsolete crates/b/t.rs:3` for `b` alone

#### Scenario: only another component's file cites it
- GIVEN the same, with no marker in `crates/b/t.rs`
- WHEN `clm pending` runs
- THEN it prints `b/gone obsolete crates/c/t.rs:8` for `c` and no line for `b`

#### Scenario: an obsolete marker in a wiring file
- GIVEN the same root with `wiring: ["crates/c/shared.rs"]`, and markers citing `b/gone` at `crates/c/shared.rs:2` and `old/t.rs:4`, which no component owns
- WHEN `clm pending` runs
- THEN it prints `b/gone obsolete crates/c/shared.rs:2 old/t.rs:4` for `b`, not blocked, and no line for `c`

#### Scenario: an obsolete marker with no wiring
- GIVEN the first scenario's repository with a root that sets no `wiring`
- WHEN `clm pending` runs
- THEN it prints `b/gone obsolete crates/b/t.rs:3 crates/c/t.rs:8` for `b`, and no line for `c`

#### Scenario: a move into the file's owner
- GIVEN a root with `wiring: []`, no component `g`, an `applied` change that removed `g/x` and added `c/x` as its move, `c/x` uncited, and a marker citing `g/x` at `crates/c/t.rs:8`, `crates/c/**` being `c`'s
- WHEN `clm pending c` runs
- THEN it prints `g/x obsolete crates/c/t.rs:8` with no `blocked`, beside `c/x <version> unbuilt`

#### Scenario: a move into the file's owner, the target blocked
- GIVEN the same, and `backlog/q.md` with `blocks: [c/x]`
- WHEN `clm pending c` runs
- THEN it prints `c/x <version> unbuilt blocked backlog/q` and `g/x obsolete crates/c/t.rs:8 blocked c/x`

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
- THEN it prints nothing

#### Scenario: a question holds an obsolete line
- GIVEN component `b`, an `applied` change that removed `b/gone`, a marker citing `b/gone` at `crates/b/t.rs:3`, and `backlog/q.md` with `kind: question` and `blocks: [b/gone]`
- WHEN `clm pending b` runs
- THEN it prints `b/gone obsolete crates/b/t.rs:3 blocked backlog/q`

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

#### Scenario: every component
- GIVEN component `a` holding `a/x`, `a/y` and `a/z`, component `a.b` holding `a.b/p`, and component `b` with status `unbuilt` holding `b/q`, each uncited
- WHEN `clm pending` runs
- THEN it prints the lines of `a/x`, `a/y`, `a/z`, `a.b/p` and `b/q`, in that order

#### Scenario: a pending requirement's priority
- GIVEN `a/x` and `a/y` uncited, changes outside the archive `m` with `priority: P2` and `k` with `priority: P4`, each modifying `a/x`, change `r` outside the archive with `priority: P3`, which removed component `g` and its requirement `g/x`, cited by markers in files of components `v` and `w`, and an archived change with `priority: P1` that added `a/y`
- WHEN `pending(graph, markers)` runs
- THEN `a/x` has priority `P2`, both lines of `g/x` `P3`, and `a/y` `P5`

#### Scenario: a merge into a requirement of the same component
- GIVEN component `a`, an `applied` change whose delta for `a` removes `a/old` and adds `a/new` and whose `moves` holds `requirement: a/old` with `to: a/new`, a marker citing `a/old` at `t.rs:3`, `a/new` uncited with `#### Needs` naming `b/z`, and `b/z` uncited
- WHEN `clm pending a` runs, and again once `b/z` is cited at its current version
- THEN it first prints `a/new <version> unbuilt blocked b/z` and `a/old obsolete t.rs:3 blocked a/new`, and then `a/new <version> unbuilt` and `a/old obsolete t.rs:3` with nothing after either

#### Serves
- capability/build-from-spec
- capability/see-project-state
- architecture/component-reconciliation

#### Needs
- model/parse-backlog
- model/human-layers
- model/reserved-ids
- model/move-records

