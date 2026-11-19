---
component: check.coverage
base: { contract: "56a3e728f960695e", design: "23d88c445cd3053c" }
---

## MODIFIED Contract
How far the code has caught up with the spec, for `check` and for every component that reads it (`check-coverage-split`). A component whose code calls this library names `check.coverage` in its `depends_on` and imports `clm-coverage`, so its build is given this Contract. `clm-check` re-exports none of it. The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library:
- `tests_in(path, text)`: the tests and named groups of tests in one file, each with its name and its lines, and `markers_outside_tests(path, text)`: the `req:` markers on none of those lines (`test-spans`).
- `markers_in(path, text)`: the `req:` markers in one file that count, those inside a test (`markers-in-tests`), none for Markdown; `scan(root)`, with the default feature `checkout`: the `req:` and `clm:remove-by` markers of every file a checkout lists, as `Scan { req, remove_by }`.
- `report(graph, markers)`: each requirement's `Coverage`, human-layer entries included (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements (`pending-report`), and `waits(lines, line)`: the lines of that list one of its lines waits on (`wait-lines`). A reader of `waits_on` takes the component of a wait from `waits`, never from the ID's own first part: an obsolete requirement is pending on the owners of the files that cite it, and its line on its own component waits on its own ID.
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)` and `complete_changes`, built on `change_progress` (`status-report`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` glob matches a repository-relative path, as `pending` and `check`'s `paths-disjoint` read `paths`.
- `uncounted(graph, outside)`, `obsolete_tests(graph, files)` and `deleted_tests(base_files, head_files)`: the markers outside tests by what they cite, the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`); `clm coverage` prints them through `clm-check` (`check/coverage-lists`).
- `uncited_tests(files)`: the tests that hold no marker, for `clm health` (`check.health/rule-signals`). `deleted_tests` counts a test as gone when the files hold fewer marker-holding tests of its name, so a test moved to another file is not, nor is one renamed in its own file that still cites every requirement it cited; `clm land` holds a code branch to it (`land/deleted-tests-approved`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## MODIFIED Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. A line is one requirement on one component, and an obsolete requirement may have several, so an ID alone names no line: `waits` resolves an entry of `waits_on` to lines, for the queue's priorities, the blockers view and the build prompt alike (`wait-lines`, `a-wait-names-a-line`). An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside comments and string and character literals, since a test that names a brace, as `find('{')` or a JSON string does, or a URL whose `//` would read as a comment, is common and was misread. The scanner reads only what it must to skip a literal or a comment: a block comment, `/* … */`, running across lines and nesting in Rust, since a quote in one, as `/* a 12" ruler */`, would otherwise open a string that never closes; Rust's `"…"` and raw `r#"…"#` strings, with any `b` or `c` prefix and running across lines, and a character literal, `'{'` or `'\''`, told from a lifetime or label (`'a`) by its closing quote; and the three quotes of TypeScript and JavaScript, a template literal's `${…}` read as part of the string and a `'` or `"` string ending at its line's end, so an apostrophe in a regular expression or JSX text costs one line, not the file. A Rust file is read for literals and block comments from its first line, so a fixture string or a commented-out test holding `#[test]` neither starts a test nor bounds the one holding it. Rust's literals and comments are then read whole, but what TypeScript and JavaScript don't read, a regular expression literal holding a quote or a parenthesis, or a template literal nested in a `${…}`, and a brace a line scanner still miscounts in either, can leave a span open, so a test's span also ends no later than the line above the first line of the next test (or, in TypeScript and JavaScript, group): a misread costs that one test's markers, which show in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never the tests after it and never a crash. The bound matters beyond the count: the mutation pass judges each marker by the innermost test holding it (`verify/mutation-pass`), so a span that ran to the end of its file had it run one test for every later marker and file their requirements unproven, though other tests proved them. A group takes no such bound, since its call holds its tests: a misread in a group's own code, outside its tests (a hook matching ``/`/``), can still run the group's span to the end of its file, which leaves its tests' spans and the innermost test of every marker as they are and costs only markers outside any test after it, counted as the group's; reading regular expressions to close that gap would mean telling one from a division and from JSX's `</`, much code for a rare case. Python's spans end by indentation and read no literal. In a file of any other extension CLM can't tell a test, so every marker counts as before; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## ADDED Requirements

### Requirement: wait-lines
`clm-coverage` SHALL offer `waits(lines, line)`, building without the `checkout` feature: for `line`, one of the lines `pending` returned as `lines`, the lines it waits on, which are, for each ID in its `waits_on` in that order, every line of `lines` whose `id` is that ID, in the order `lines` holds them, other than `line` itself, a line being itself when both its `id` and its component are `line`'s. So an obsolete requirement's line on its own component, which waits on its own ID (`pending-report`), waits on that requirement's lines on the other components, and a wait on a requirement pending on the line's own component is returned like any other. It SHALL return nothing for a line whose `waits_on` is empty, and nothing for an ID no line of `lines` holds.

#### Scenario: an obsolete requirement's other lines
- GIVEN a root with `wiring: []`, components `b`, `c` and `d` with `paths` `["crates/b/**"]`, `["crates/c/**"]` and `["crates/d/**"]`, an `applied` change that removed `b/gone`, and markers citing `b/gone` in a file of each
- WHEN `waits` runs for `b/gone`'s line on `b`, and for its line on `c`
- THEN the first returns its lines on `c` and `d`, in that order, and the second returns nothing

#### Scenario: a need
- GIVEN `a/y` uncited with `#### Needs` naming `b/z`, and `b/z` uncited
- WHEN `waits` runs for `a/y`'s line
- THEN it returns `b/z`'s line, on `b`

#### Scenario: a wait on the line's own component
- GIVEN a root with `wiring: []`, no component `g`, an `applied` change that removed `g/x` and added `c/x` as its move, `c/x` uncited and in the `blocks` of `backlog/q.md`, and a marker citing `g/x` in a file of `c`
- WHEN `waits` runs for `g/x`'s line on `c`
- THEN it returns `c/x`'s line, on `c`

#### Serves
- capability/build-from-spec
- architecture/component-reconciliation

## MODIFIED Requirements

### Requirement: pending-report <!-- base: ebf1b50f9ebe8c09 -->
`clm pending [<component>]` SHALL print one line per pending requirement of every component, or of `<component>`, sorted by component ID then requirement ID: `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` with the versions its markers cite other than its current one (`none` for a marker without one), sorted, or `<id> obsolete <file>:<line>[ ...]`; each followed by ` blocked <blocker>[,<blocker>...]` when it is blocked, each blocker `backlog/<slug>` for a backlog item that names it in `blocks`, on each of its lines when it is `obsolete` (`model/parse-backlog`) or `<component>/<slug>` for a pending requirement its `Needs` name (`requirement-needs`), for an `obsolete` one a pending requirement a move took it to, or for a capability a requirement serving it that holds it back, items first, each group sorted. An `obsolete` requirement SHALL be blocked by each pending requirement that the moves `status-report` defines take it to, followed as `progress-follows-moves` follows them to the IDs the spec holds, on whatever component each is (`moved-markers-wait`): the build of that requirement writes its own tests in its own component's files, so the obsolete line waits for it, and the components the obsolete requirement is pending on then remove its markers and tests, and its own component its code (`coupled-changes-sequence`). A capability that is pending SHALL count as pending on the component that builds the capabilities' tests, which `pending` takes from the model's `Graph::capability_tests_node` (`model/capability-tests-component`) rather than deciding itself, and is blocked like any requirement by the items whose `blocks` name it. A pending capability, `unbuilt` or `behind`, SHALL also be blocked by each requirement whose effective serves (`model/serves`) names it, on whatever component, optional or not, that is `unbuilt`, or `behind` and itself blocked, by an item whose `blocks` names it or by any requirement in its own `waits_on`, whatever that requirement's state and whatever blocks it in turn; and by no current one, nor any `behind` one that nothing blocks (`serving-rules-order-capabilities`). So its test is built after the rules it exercises, and isn't offered while a changed rule it exercises waits on other work. A capability holding `#### Untested` is never pending, and neither is an architecture entry: the rules its `#### Enforced by` names are pending on their own components (`human-layer-proof`). It SHALL print nothing when nothing is pending. The command's exit status is `view.repo/arguments-and-exit`'s, and narrowing the list to `<component>`, with the refusal of one the spec graph doesn't hold, is `check/pending-narrows-on-a-graph`'s: no test of this requirement SHALL be left in `crates/clm/tests/pending.rs` or in `check`'s files (`commands-live-in-their-crates`). An `obsolete` marker whose ID names a component the spec graph doesn't hold SHALL count as pending on the component `owner` gives for the marker's file, or on the root `clm` when it gives none, however the component went: removed by a delta with `remove: true` (`node-removal`) or gone without one. A removed requirement cited in files of several components is pending on each of them, each line listing only that component's files. Once the root sets `wiring` (`model/wiring-globs`), an `obsolete` requirement whose component the spec graph holds SHALL also be pending on each other component that owns a file citing it, that component's line listing only its files, while a file a `wiring` glob matches and a file `owner` gives no component for count as the requirement's own component's. Its line on its own component, which lists those files and that component's, SHALL then be blocked by its own ID, standing for its lines on the other components, while any of them is left, `pending` returning that ID in the line's `waits_on` and in no other line's (which lines an entry of `waits_on` stands for is `wait-lines`'s), so the build that deletes a test, which breaks nothing, comes before the build that removes the code the test ran; when its own component has no such file it has no line. A blocker that `moved-markers-wait` gives an obsolete line SHALL be left out of a line pending on that blocker's own component when nothing blocks that blocker, its own `blocked_by` and `waits_on` both empty, since that component's one build then makes it current; a blocker that is itself blocked SHALL stay on that line too, so the build re-cites no test to a requirement it was told to leave out. A removed capability is placed as `removed-capabilities` places it. While the root sets no `wiring`, an obsolete requirement whose component the graph holds SHALL stay pending on that component alone, every file listed. `pending(graph, markers)` SHALL return the same for a graph and markers loaded without a checkout, with each requirement's priority, the slugs of the items that block it in `blocked_by`, and its other blockers, as `<component>/<slug>` in the order `clm pending` prints them, in `waits_on`: the requirements its `Needs` name, those a move took an `obsolete` one to, and the rules a capability waits on, in one list (`priority-orders-nodes` seeds through each).

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

#### Serves
- capability/build-from-spec
- capability/see-project-state
- architecture/component-reconciliation

#### Needs
- model/parse-backlog
- model/human-layers
- model/reserved-ids

### Requirement: removed-capabilities <!-- base: d9e27527c845005e -->
A capability SHALL count as *removed* while a change outside the archive, whatever its status, lists its slug under `removes_capabilities` (`model/parse-changes`) and `CAPABILITIES.md` holds no entry of that slug: `removed(graph)` SHALL give `capability/<slug>` with the IDs of those changes, sorted, as it gives a requirement a delta removed, and SHALL NOT give a listed capability that `CAPABILITIES.md` holds, which is a capability like any other. A marker citing a removed capability SHALL read as `obsolete`, as one citing a removed requirement does: `clm coverage` prints `capability/<slug> obsolete <file>:<line>[ ...]` among its obsolete lines (`coverage-report`), and `clm pending` prints the same line, with every marker's location, where `pending-report` places an obsolete requirement: when the graph holds the component `capability`, as a requirement of that component, so that once the root sets `wiring` (`model/wiring-globs`) a marker in a file another component owns is on that component's line and the line on `capability` is blocked by the capability's own ID while one is left; and otherwise on the owner of each marker's file, or the root. No need, move or serving rule can name a capability that is gone, so only a backlog item whose `blocks` names it (`model/parse-backlog`) blocks the line, and its priority is the highest `priority` among the changes that list it, as a requirement a delta removes takes its change's. So a spec branch that removes a cited capability by hand passes `clm check` (`check/dangling-markers`), and the build of the component the line is pending on removes the tests (`removed-capabilities-build-out`), or files the `question` that holds the line where it can neither delete nor re-cite one (`orchestrator.run/pending-prompt`). Once the change is archived, the capability is no longer removed and its markers dangle, so the change can't be complete while one is left (`status-report`).

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

#### Scenario: removed capability in another component's file
- GIVEN a root with `wiring: []`, component `capability` with `paths` `["crates/clm/tests/capability/**"]`, component `c` with `paths` `["crates/c/**"]`, a `CAPABILITIES.md` with no entry `gone`, an `applied` change with `removes_capabilities: [gone]`, and markers citing `capability/gone` at `crates/clm/tests/capability/gone.rs:3` and `crates/c/t.rs:8`
- WHEN `clm pending` runs, and again once the marker in `crates/c/t.rs` is gone
- THEN the first prints `capability/gone obsolete crates/clm/tests/capability/gone.rs:3 blocked capability/gone` for `capability` and `capability/gone obsolete crates/c/t.rs:8` for `c`, and the second prints `capability/gone obsolete crates/clm/tests/capability/gone.rs:3` for `capability` alone

#### Scenario: a question holds a removed capability
- GIVEN the first scenario's files, and `backlog/q.md` with `kind: question` and `blocks: [capability/gone]`
- WHEN `clm pending capability` runs
- THEN it prints `capability/gone obsolete capabilities/web/gone.spec.ts:5 crates/clm/tests/capability/gone.rs:3 blocked backlog/q`

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- model/parse-changes
- model/parse-backlog
- model/human-layers
- model/reserved-ids
