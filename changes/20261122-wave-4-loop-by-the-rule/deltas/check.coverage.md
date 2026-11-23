---
component: check.coverage
base: { design: "96ec8c48dd75ec3d" }
---

## MODIFIED Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status`, `lists`, `spans` and `glob`. Its readers import it directly (`check-coverage-split`): `clm coverage` and `clm pending` reach it through `clm-check`, and `clm status` through `clm-view`. A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers: `clm:remove-by ` not preceded by an ASCII letter, digit or `_`, followed by a change ID of eight ASCII digits, `-` and one or more of `a-z`, `0-9` and `-`, the marker `check.unreferenced/removal-marked` accepts, so every item that detector skips for a marker has its marker checked by `check/remove-by-markers` (`removal-markers`). A `req:` marker's word character is any Unicode letter or digit, or `_`; the removal marker's is ASCII alone, as `clm unreferenced`, which depends on `vcs` alone, reads it. Where `git ls-files` fails, as outside a git work tree such as a tree `clm wrap-up` extracts, every file there is tracked, so `scan` walks the files under the root instead, leaving out only a `.git` directory and `target`, as `rename` reads a tree with no `.git`, and reading a directory entry's own type, so a link to a directory is never followed and a cycle of links ends (`markers-from-files`). One call, `read_files`, gives those files and their texts, and `scan` reads them through it; `clm-check` reads the files for `clm coverage --obsolete-tests` and `--deleted-tests` through it too, so the two never list different files (`check/coverage-lists`, `a-wait-names-a-line`). `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. A line is one requirement on one component, and an obsolete requirement may have several, so an ID alone names no line: `waits` resolves an entry of `waits_on` to lines, for the queue's priorities, the blockers view and the build prompt alike (`wait-lines`, `a-wait-names-a-line`). An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Status** (`status-report`). `status_report` gives `clm status`'s rows as cells and carries no text form of its own: `view/status-report` lays `clm status` out, the `none` of an empty section included, so the report has no `Display` and nothing here writes the text.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside comments and string and character literals, since a test that names a brace, as `find('{')` or a JSON string does, or a URL whose `//` would read as a comment, is common and was misread. The scanner reads only what it must to skip a literal or a comment: a block comment, `/* … */`, running across lines and nesting in Rust, since a quote in one, as `/* a 12" ruler */`, would otherwise open a string that never closes; Rust's `"…"` and raw `r#"…"#` strings, with any `b` or `c` prefix and running across lines, and a character literal, `'{'` or `'\''`, told from a lifetime or label (`'a`) by its closing quote; and the three quotes of TypeScript and JavaScript, a template literal's `${…}` read as part of the string and a `'` or `"` string ending at its line's end, so an apostrophe in a regular expression or JSX text costs one line, not the file. A Rust file is read for literals and block comments from its first line, so a fixture string or a commented-out test holding `#[test]` neither starts a test nor bounds the one holding it. Rust's literals and comments are then read whole, but what TypeScript and JavaScript don't read, a regular expression literal holding a quote or a parenthesis, or a template literal nested in a `${…}`, and a brace a line scanner still miscounts in either, can leave a span open, so a test's span also ends no later than the line above the first line of the next test (or, in TypeScript and JavaScript, group): a misread costs that one test's markers, which show in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never the tests after it and never a crash. The bound matters beyond the count: the mutation pass judges each marker by the innermost test holding it (`verify/mutation-pass`), so a span that ran to the end of its file had it run one test for every later marker and file their requirements unproven, though other tests proved them. A group takes no such bound, since its call holds its tests: a misread in a group's own code, outside its tests (a hook matching ``/`/``), can still run the group's span to the end of its file, which leaves its tests' spans and the innermost test of every marker as they are and costs only markers outside any test after it, counted as the group's; reading regular expressions to close that gap would mean telling one from a division and from JSX's `</`, much code for a rare case. Python's spans end by indentation and read no literal. In a file of any other extension CLM can't tell a test, so every marker in it counts; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## MODIFIED Requirements

### Requirement: status-report <!-- base: 3f658f05488ae7c1 -->
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status as `active`, `unbuilt` or `deprecated`, and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds, nor a capability it lists under `removes_capabilities` that `CAPABILITIES.md` no longer holds (`removed-capabilities`). For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. The command's exit status is `view.repo/arguments-and-exit`'s.

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

