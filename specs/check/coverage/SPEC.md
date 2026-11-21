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
How far the code has caught up with the spec, for `check` and for every component that reads it (`check-coverage-split`). A component whose code calls this library names `check.coverage` in its `depends_on` and imports `clm-coverage`, so its build is given this Contract. `clm-check` re-exports none of it. The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library:
- `Marker { id, version, path, line }`: a marker at a file's line (1-based), `version` none for a bare marker; it displays as `<file>:<line>`. `State`, one of `Current`, `Behind`, `Unbuilt` and `Obsolete`, and `STATE_NAMES`, each `State` with the word the reports print for it (`state-names`).
- `tests_in(path, text)`: the tests and named groups of tests in one file, each a `TestSpan { name, group, first, last }`, and `markers_outside_tests(path, text)`: the `req:` markers on none of those lines (`test-spans`). `marker_spans(text)`: every `req:` marker in a text, inside a test or not, each a `MarkerSpan { id, version, range }` with its byte range (`marker-spans`).
- `markers_in(path, text)`: the `req:` markers in one file that count, those inside a test (`markers-in-tests`), none for a file whose extension is `md`; `scan(root)`, with the default feature `checkout`: the markers of every file a checkout lists, or of the files under `root` where `git ls-files` fails (`markers-from-files`), as `Scan { req, outside, remove_by }`: the `req:` markers that count, those on no line of a test (`test-spans`), and the `clm:remove-by` markers, each `id` a change's.
- `report(graph, markers)`: each requirement's `Coverage { id, state, version, markers, proof }`, human-layer entries included, an entry that stands by its `#### Enforced by` or `#### Untested` block carrying its `Proof`; each displays as its `clm coverage` line (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements, each a `Pending { id, node, version, state, cited, markers, priority, blocked_by, waits_on }`, `node` the component its line is pending on, displaying as its `clm pending` line (`pending-report`), and `waits(lines, line)`: the lines of that list one of its lines waits on (`wait-lines`). A reader of `waits_on` takes the component of a wait from `waits`, never from the ID's own first part: an obsolete requirement is pending on the owners of the files that cite it, and its line on its own component waits on its own ID.
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)`: `clm status`'s `components` and `changes` sections as `StatusReport { nodes, changes }`, each row a `Row` of cells, and `complete_changes(graph, markers)`: the IDs of the changes that are complete, both built on `change_progress` (`status-report`). `moved_to(graph, id)`: the requirements the moves take a removed requirement or capability to (`move-targets`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` or `tests` glob matches a repository-relative path, as `owner`, `pending` and every other reader of those keys match.
- `uncounted(graph, outside)`, each an `Uncounted { id, markers }`, `obsolete_tests(graph, files)`, each an `ObsoleteTest { test, line, ids, every }`, and `deleted_tests(base_files, head_files)`, each a `DeletedTest { test, line, ids }`: the markers outside tests by what they cite, the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`); `clm coverage` prints them through `clm-check` (`check/coverage-lists`). `bare_name(name)`: a test's name with its markers left out, as `deleted_tests` compares names (`test-lists`).
- `uncited_tests(files)`, each an `UncitedTest { test, line }`: the tests that hold no marker, for `clm health` (`check.health/rule-signals`). `deleted_tests` counts a test as gone when the files hold fewer marker-holding tests of its name, so a test moved to another file is not, nor is one renamed in its own file that still cites every requirement it cited; `clm land` holds a code branch to it (`land/deleted-tests-approved`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status`, `lists`, `spans` and `glob`. Its readers import it directly (`check-coverage-split`): `clm coverage` and `clm pending` reach it through `clm-check`, and `clm status` through `clm-view`. A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Where `git ls-files` fails, as outside a git work tree such as a tree `clm wrap-up` extracts, every file there is tracked, so `scan` walks the files under the root instead, leaving out only a `.git` directory and `target`, as `rename` reads a tree with no `.git` (`markers-from-files`). `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. A line is one requirement on one component, and an obsolete requirement may have several, so an ID alone names no line: `waits` resolves an entry of `waits_on` to lines, for the queue's priorities, the blockers view and the build prompt alike (`wait-lines`, `a-wait-names-a-line`). An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside comments and string and character literals, since a test that names a brace, as `find('{')` or a JSON string does, or a URL whose `//` would read as a comment, is common and was misread. The scanner reads only what it must to skip a literal or a comment: a block comment, `/* … */`, running across lines and nesting in Rust, since a quote in one, as `/* a 12" ruler */`, would otherwise open a string that never closes; Rust's `"…"` and raw `r#"…"#` strings, with any `b` or `c` prefix and running across lines, and a character literal, `'{'` or `'\''`, told from a lifetime or label (`'a`) by its closing quote; and the three quotes of TypeScript and JavaScript, a template literal's `${…}` read as part of the string and a `'` or `"` string ending at its line's end, so an apostrophe in a regular expression or JSX text costs one line, not the file. A Rust file is read for literals and block comments from its first line, so a fixture string or a commented-out test holding `#[test]` neither starts a test nor bounds the one holding it. Rust's literals and comments are then read whole, but what TypeScript and JavaScript don't read, a regular expression literal holding a quote or a parenthesis, or a template literal nested in a `${…}`, and a brace a line scanner still miscounts in either, can leave a span open, so a test's span also ends no later than the line above the first line of the next test (or, in TypeScript and JavaScript, group): a misread costs that one test's markers, which show in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never the tests after it and never a crash. The bound matters beyond the count: the mutation pass judges each marker by the innermost test holding it (`verify/mutation-pass`), so a span that ran to the end of its file had it run one test for every later marker and file their requirements unproven, though other tests proved them. A group takes no such bound, since its call holds its tests: a misread in a group's own code, outside its tests (a hook matching ``/`/``), can still run the group's span to the end of its file, which leaves its tests' spans and the innermost test of every marker as they are and costs only markers outside any test after it, counted as the group's; reading regular expressions to close that gap would mean telling one from a division and from JSX's `</`, much code for a rare case. Python's spans end by indentation and read no literal. In a file of any other extension CLM can't tell a test, so every marker in it counts; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## Requirements

### Requirement: coverage-report
`clm coverage` SHALL print one line per requirement, and per human-layer entry (`architecture/<slug>`, `capability/<slug>`, `model/human-layers`), all sorted together by ID: `<id> current <file>:<line>[ <file>:<line>...]` when markers cite it, every one cites its current version and no backlog item's `unproven` names it at that version (`model/parse-backlog`), `<id> behind <file>:<line>[ ...]` listing the markers that cite it at another version or without one when any does, even beside markers at its current version, which the line leaves out (`stale-markers-keep-behind`), otherwise `<id> unbuilt`, which a requirement that an item's `unproven` names at its current version reads whatever cites it, since the mutation pass found that its tests notice no change to its code (`verify/mutation-pass`). An architecture entry is never cited (`check/dangling-markers`): its line is `<id> current` when it holds an `#### Enforced by` block and every rule it names is current, `<id> pending <rule>[,<rule>...]` naming, in block order, the rules it names that aren't, `<id> untested` when it holds only an `#### Untested` block, and `<id> unenforced` when it holds neither, which `check/unenforced` fails; a capability holding `#### Untested` is `<id> untested` (`human-layer-proof`); and, after those, one line `<id> obsolete <file>:<line>[ <file>:<line>...]` per requirement that no longer exists but that a change outside the archive removed and a marker still cites.

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
For any checkout, `report` over the union of `markers_in` applied to each tracked file SHALL equal the coverage `clm coverage` reports, versions included, and `markers_in` SHALL return nothing for a Markdown file, a file whose extension is exactly `md`. `markers_in`, `markers_outside_tests`, `marker_spans` and `scan` SHALL read a marker's version as the `@` directly after its slug followed by 8 characters each one of `0`-`9` and `a`-`f`, the character after them, if any, not one of those; with any other text after its slug, or none, the marker carries no version. `scan(root)` SHALL read the files `git ls-files --cached --others --exclude-standard` lists in `root`, and, where that fails, as it does outside a git work tree, every file under `root` outside any directory named `.git` or `target`; a file it can't read, or whose bytes aren't UTF-8, SHALL give no marker.

#### Scenario: same as checkout
- GIVEN a checkout with a test file citing one requirement at its current version and another at an older one, and a Markdown file citing a third
- WHEN coverage is computed with `markers_in` and `report` from the files' paths and texts, and with `clm coverage`
- THEN both report the first requirement current and the second behind at the same locations, and the third unbuilt

#### Scenario: a version a marker doesn't carry
- GIVEN a test's line in `t.rs` holding `req:a/x@0123abcd`, `req:a/y`, `req:a/z@xyz`, `req:a/w@0123abcd9`, `req:a/v@0123ABCD` and `req:a/u@0123abcdg`
- WHEN `markers_in` runs on it
- THEN `a/x` and `a/u` carry the version `0123abcd`, and `a/y`, `a/z`, `a/w` and `a/v` carry none

#### Scenario: a tree outside git
- GIVEN a directory in no git work tree holding `t.rs` with a test citing `a/x`, `.hidden/t.rs` with a test citing `a/w`, `.git/t.rs` and `target/t.rs` each with a test citing `a/y`, and `b.rs`, whose bytes aren't UTF-8, with a test citing `a/z`
- WHEN `scan` runs on it
- THEN its `req` holds the markers citing `a/w` and `a/x` and no other

#### Serves
- capability/follow-from-browser
- capability/prove-by-tests
- architecture/git-is-truth

### Requirement: pending-report
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

#### Serves
- capability/build-from-spec
- capability/see-project-state
- architecture/component-reconciliation

#### Needs
- model/parse-backlog
- model/human-layers
- model/reserved-ids

### Requirement: status-report
`clm status` SHALL hold, from the working tree only, two sections laid out as `view`'s `status-report` says: `components`, one row per component sorted by ID, with its ID, its status and `<current>/<total>` (requirements current / all requirements; the capabilities that don't hold `#### Untested`, current or not, are counted on the row of the component they are pending on when pending, `capability` or else the root, `pending-report`, and the architecture entries on no row, since their rules are counted on their own components); and `changes`, which groups the changes outside the archive by status, in the order `implementing`, `applied`, `approved`, `proposed`, then any other in sorted order, each group one row per change sorted by ID, with its ID and `<current>/<total> requirements` (requirements its deltas add or modify that are current / all of them), followed by `complete` when the change is complete: its status is `applied` or `implementing`, all of them are current, and no marker cites a requirement its deltas remove that the spec no longer holds, nor a capability it lists under `removes_capabilities` that `CAPABILITIES.md` no longer holds (`removed-capabilities`). For an `applied` or `implementing` change, a requirement the spec no longer holds SHALL be counted as each requirement a move took it to (`progress-follows-moves`): a change whose status is `applied`, `implementing` or `done`, inside or outside the archive, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes `<a>/<slug>` and another adds `<b>/<slug>` to a component other than `<a>`, as a rename does (`node-removal`); moves are followed again while the spec doesn't hold the ID they reach, each ID once. Both counts leave out the requirements that no move takes to one the spec holds, since a later change removed them; a `proposed` or `approved` change hasn't applied, so it counts all of them. A requirement a later change added back under the same ID is a new requirement, whose markers, at whatever version, don't hold the earlier change back (`archive-on-sync`). The spec and the markers are read from the same tree: the working tree for `clm status`, and whatever tree the caller of `complete_changes` loaded, such as `HEAD` for `clm rebase` and `origin/main` for `clm wrap-up`. A section with no rows has the single row `none`. The command's exit status is `view.repo/arguments-and-exit`'s.

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
`owner(graph, path)` SHALL return the component whose `tests` match `path` (`model/test-ownership-keys`, `components-own-their-tests`), or, when none's do, the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `tests` or `paths` do, matching each glob with `matches`; when two components' `tests` match, it SHALL return the first by ID, which `check/paths-disjoint` reports. `pending` SHALL decide a file's component with this `owner`, falling back to the root when it returns none. `matches(pattern, path)` SHALL match `path` against one glob as `pending` matches `paths` globs (git's `:(glob)` pathspec semantics).

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Scenario: tests inside another component's paths
- GIVEN component `harness` with `paths: ["tests/**"]` and component `a` with `tests: ["tests/a/**"]`
- WHEN `owner` runs on `tests/a/conftest.py` and `tests/helpers.py`
- THEN it returns `a` and `harness`

#### Scenario: one glob
- GIVEN the pattern `crates/*/tests/**`
- WHEN `matches` runs on `crates/a/tests/t.rs` and `crates/a/src/t.rs`
- THEN it returns true and false

#### Scenario: globs as git reads them
- GIVEN a repository's tracked files and a `paths` glob that uses `*`, `**` or `?`
- WHEN `matches` runs on the glob and each tracked file
- THEN it returns true for exactly the files `git ls-files ':(glob)<glob>'` lists

#### Serves
- capability/work-concurrently
- capability/build-from-spec
- architecture/path-ownership

#### Needs
- model/test-ownership-keys

### Requirement: state-names
`clm-coverage` SHALL offer the public constant `STATE_NAMES`, pairing each requirement `State` with the word `clm coverage` and `clm pending` print for it (`current`, `behind`, `unbuilt`, `obsolete`). The list SHALL hold every variant of `State` exactly once.

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

### Requirement: test-spans
`clm-coverage` SHALL offer `tests_in(path, text)`, returning the tests and the named groups of tests in one file's text, in the order they start, each with its name, whether it is a group, and its first and last line, the first being the first of the comment and attribute lines directly above it with no blank or code line between, and SHALL find them by the file's extension:
- `.rs`: a test is a function that an attribute precedes whose path's last segment is `test`, `rstest` or `test_case` or ends `_test` (`#[test]`, `#[tokio::test]`, `#[wasm_bindgen_test]`), named by the function, from those lines to the line that closes its body, found by counting braces outside line comments, block comments (`/* … */`, nesting) and string and character literals, and ending no later than the line above the next test's first line, the end of the file when no test follows; the reading of literals and block comments carries across lines, so a test attribute inside a string or a block comment starts no test; there are no groups. A comment line between the attribute and the function is one of its lines.
- `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs`: a test is a call of `it` or `test`, and a group a call of `describe` or `test.describe`, each with or without the modifiers `check/skipped-tests` reads and `.only`, `.each`, `.concurrent` and `.serial`, starting a line after leading whitespace, named by its first argument when that is a string literal and by an empty name otherwise, from those lines to the line that closes the call, found by counting its parentheses outside line comments, block comments (`/* … */`) and string and template literals, a `'` or `"` string ending at the end of its line and a block comment and a template literal running across lines; a test's or a group's first lines never reach back onto the line of the call before it, and a test's span, not a group's, ends no later than the line above the first line of the next test or group, the end of the file when none follows. Each call's reading starts on its own line, so a call starts a test or a group whatever literal or block comment a line above it left open.
- `.py`: a test is a function, `def` or `async def`, whose name starts `test`, and a group a class whose name starts `Test`, named by it alone, from the decorator and comment lines directly above it to the last line indented deeper than its `def` or `class` line.
- any other extension: none.

It SHALL offer `markers_outside_tests(path, text)`, returning each marker `marker_spans` finds in `text` on a line that belongs to no test or group, as `Marker`s, and nothing for a Markdown file or a file of another extension than those above. `scan(root)` SHALL return them for every file it reads as `outside`, beside `req`. Both calls SHALL build without the `checkout` feature. Which markers `markers_in` returns and `scan`'s `req` holds is `markers-in-tests`'s to say, not this requirement's: `outside` holds the markers outside tests whether or not `req` holds them too.

#### Scenario: a Rust test and a helper
- GIVEN the text of `t.rs`: a comment line holding `req:a/x`, then `#[test]`, then `fn proves_x() {`, a line holding `req:a/y`, `}`, a blank line, and `fn helper() {` with a line holding `req:a/z` and `}`
- WHEN `tests_in` and `markers_outside_tests` run on it
- THEN `tests_in` returns one test, `proves_x`, from line 1 to line 5, and `markers_outside_tests` returns `a/z` alone

#### Scenario: braces in literals
- GIVEN the text of `t.rs`: `#[test]`, `fn a() {`, `    let at = rest.find('{');`, `    let url = "https://x/{";`, `}`, `#[test]`, `fn b() {`, a line holding `req:a/y` and `}`
- WHEN `tests_in` runs on it
- THEN it returns `a` from line 1 to line 5 and `b` from line 6 to line 9

#### Scenario: a brace left open
- GIVEN the text of `t.rs`: `#[test]`, `fn a() {`, `    if x {`, `}`, a comment line holding `req:a/y`, `#[test]`, `fn b() {` and `}`
- WHEN `tests_in` runs on it
- THEN it returns `a` from line 1 to line 4 and `b` from line 5 to line 8

#### Scenario: a quote in a block comment
- GIVEN the text of `t.rs`: `/* a 12" ruler`, `   /* nested */ #[test] fn c() {`, `*/`, `#[test]`, `fn a() {`, `}`, `#[test]`, `fn b() {`, a line holding `req:a/y` and `}`
- WHEN `tests_in` runs on it
- THEN it returns `a` from line 1 to line 6 and `b` from line 7 to line 10, and no test `c`

#### Scenario: a backtick in a block comment
- GIVEN the text of `a.test.ts`: `describe('g', () => {`, ``  /* a ` */``, `  it('a', () => {});`, `});` and `it('b', () => {});`
- WHEN `tests_in` runs on it
- THEN it returns the group `g` from line 1 to line 4, `a` from line 2 to line 3 and `b` from line 5 to line 5

#### Scenario: a backtick the scanner doesn't read
- GIVEN the text of `a.test.ts`: `it('a', () => {`, ``  expect(s).toMatch(/`/);``, `});`, `it('b', () => {`, a line holding `req:a/y` and `});`
- WHEN `tests_in` runs on it
- THEN it returns `a` from line 1 to line 3 and `b` from line 4 to line 6

#### Scenario: a parenthesis in a string
- GIVEN the text of `a.test.ts`: `it('a', () => {`, `  f(')');`, `});`, a blank line and a line holding `req:a/y`
- WHEN `tests_in` and `markers_outside_tests` run on it
- THEN `tests_in` returns `a` from line 1 to line 3, and `markers_outside_tests` returns `a/y` alone

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

#### Scenario: a test in a block comment
- GIVEN the text of `a.test.ts`: `/*`, `it('a', () => {});`, `*/` and `it('b', () => {});`
- WHEN `tests_in` runs on it
- THEN it returns `a` from line 1 to line 2 and `b` from line 3 to line 4

### Requirement: markers-in-tests
`markers_in(path, text)` and `scan(root)`'s `req` SHALL hold, for a file of an extension `test-spans` reads, only the markers on a line of a test or a group of tests, and for any other file every marker, none for a file whose extension is exactly `md`, so a marker in code that isn't a test proves nothing (`markers-count-in-tests`).

#### Scenario: a marker in code that is no test
- GIVEN `a/x`, and `src/lib.rs` holding `req:a/x` at its current version in the body of a function with no test attribute, cited nowhere else
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt`

#### Scenario: a marker in a test
- GIVEN the same marker in the body of a `#[test]` function in `tests/x.rs`
- WHEN `clm coverage` runs
- THEN it prints `a/x current tests/x.rs:<line>`

#### Scenario: a file-header marker
- GIVEN `a.test.ts` whose first line is a comment holding `req:a/x`, followed by a blank line and tests
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt`

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
- check.affected/affected-files
- check.health/churn-signal
- rename/marker-rewrite

### Requirement: test-lists
`clm-coverage` SHALL offer, building without the `checkout` feature and taking files as paths and texts: `uncounted(graph, outside)`, returning, sorted by ID, each requirement or capability that exists, or that a change outside the archive removed (`removed`), and that a marker outside a test cites, never an architecture entry, with those markers' locations; `obsolete_tests(graph, files)`, returning each test (`test-spans`, groups left out) that holds a marker citing a requirement or capability a change outside the archive removed, sorted by file and then by line, with the removed IDs it cites, sorted, and whether every marker on the test's lines cites one; `deleted_tests(base_files, head_files)`, returning the tests of `base_files` that are *gone*, sorted the same way, with the IDs each cited; and `uncited_tests(files)`, returning each test (groups left out) with no marker on its lines, sorted the same way, with its first line. A test is named `<file>::<name>`. For `deleted_tests` a test's name is compared with every `req:` marker in it left out, ID and version, and the space before it, so re-citing a test whose title carries its marker doesn't rename it; `clm-coverage` SHALL offer that as `bare_name(name)`. A file's *gained* tests of a bare name number how many fewer of its tests of that name holding no marker the head's text of the file holds than its text in `base_files` does, but no more than the tests of that name holding a marker the head's text holds: that many of its citing tests at the head may be unmarked tests that gained a marker, and stand for no test the base held with one. Where `base_files` hold `n` tests of one bare name that each hold a marker and `head_files` hold `m` such tests less every file's gained tests of that name, `m` less than `n`, `n - m` tests are gone: taking the files in sorted order, each file gives as many of its tests of that name holding a marker, last first, as its tests of that name holding a marker at the head, less its gained tests of that name, fall short of those at the base, until `n - m` are taken; where `m` is `n` or more, none is gone. A test so taken *fits* a test of the head's text of its file that holds a marker, whose bare name no test of `base_files` (groups left out) carries, with a marker or without, and whose markers cite every ID the taken test's markers cite, versions left out. A test so taken SHALL instead count as *renamed*, and not be returned, as follows: taking the tests so taken in the order `deleted_tests` returns them, each counts as renamed when it and every earlier one counted renamed can each be paired with a different test it fits. So as many are renamed as can be, each head test standing for one at most. Here groups are left out on both sides. A test of the head's text of a file is *new* when no test of `base_files` carries its bare name, with a marker or without. A file's *stripped* tests of a bare name number how many fewer of its tests of that name holding a marker the head's text of the file holds than its text in `base_files` does, but no more than the tests of that name holding no marker the head's text holds: that many of its unmarked tests at the head may be cited tests that lost their marker, and stand for no test the base held without one. A file's *gone unmarked* tests number, for each bare name, how many more of its tests of `base_files` holding no marker carry that name than the head's text of the same file accounts for, which is its gained tests of that name added to what is left of its tests of that name there holding no marker once its stripped tests of that name are taken off; none where it is no more. A gone unmarked test could have become any new test of its file, so it takes one before a taken test does: a taken test SHALL count as renamed only when, with it counted, the tests counted renamed in its file would number no more than the file's new tests less its gone unmarked tests, and the tests of its bare name counted renamed in its file would number no more than how many fewer tests of that name, with a marker or without, the head's text of the file holds than its text in `base_files` does: a test whose name its file still carries as often as before was not renamed. A test moved to another file under its name is not gone, unless that file's gained tests of the name take its place, nor is one renamed in its own file that cites at least what it cited; a test renamed and moved to another file at once, one renamed that cites less, one whose place a test the base held with no marker takes by gaining one, under its name or a new one, a deleted one, and one that loses its last marker each are. Of two taken tests that a head test both fits, one deleted and one renamed, the trees can't tell which is which, and the earlier counts as renamed. Where a file's text in `base_files` holds a test of a name with a marker and one of that name with none, and its head's text holds only one test of that name, with a marker, the trees can't tell the unmarked one deleted from the cited one deleted as the other gains a marker, and the cited one counts as gone. Where it holds a test of a name with a marker and one of that name with none, and its head's text holds only one test of that name, with no marker, beside a new test that holds a marker, the trees can't tell the cited one renamed beside the unmarked one kept from the cited one losing its last marker as the unmarked one is deleted, and the cited one counts as gone.

#### Scenario: uncounted
- GIVEN `a/x` existing, `a/gone` removed by an `applied` change, and markers outside a test citing `a/x` at `src/lib.rs:3` and `a/gone` at `src/lib.rs:9`
- WHEN `uncounted` runs
- THEN it returns `a/gone` with `src/lib.rs:9`, then `a/x` with `src/lib.rs:3`

#### Scenario: an entry outside a test
- GIVEN `a/x` existing, `architecture/e` an entry of `ARCHITECTURE.md`, and markers outside a test citing `architecture/e` at `src/lib.rs:2` and `a/x` at `src/lib.rs:3`
- WHEN `uncounted` runs
- THEN it returns `a/x` with `src/lib.rs:3` alone

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `only_x` citing `a/x` alone and a test `both` citing `a/x` and `a/y`, both in `tests/t.rs`, `both` first
- WHEN `obsolete_tests` runs
- THEN it returns `tests/t.rs::both` with `a/x`, not every marker, then `tests/t.rs::only_x` with `a/x`, every marker

#### Scenario: a deleted test
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and head files whose `tests/t.rs` holds only `one`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: a renamed test
- GIVEN the same, with `two` renamed `second` at the head, still citing `a/x`
- WHEN it runs
- THEN it returns nothing

#### Scenario: a renamed test that cites less
- GIVEN base files whose `tests/t.rs` holds tests `one` citing `a/x` and `two` citing `a/x` and `a/y`, and head files whose `tests/t.rs` holds `one` and `second`, `second` citing `a/x` alone
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x` and `a/y`

#### Scenario: a test renamed into another file
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds only `one` and whose `tests/u.rs` holds `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: one new name for two gone tests
- GIVEN base files whose `tests/t.rs` holds tests `one`, `two` and `three`, each citing `a/x`, and head files whose `tests/t.rs` holds `one` and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::three` alone

#### Scenario: a test that gains its first marker
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `quiet`, `quiet` now citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: an uncited test renamed as it gains its first marker
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `quiet_now`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: an uncited test renamed to a name it doesn't cite under
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one`, `second`, citing `a/x`, and `quiet_now`, with no marker
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: an uncited test deleted beside a rename
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: two renames that each keep their citations
- GIVEN base files whose `tests/t.rs` holds tests `first` citing `a/x` and `second` citing `a/x` and `a/y`, and head files whose `tests/t.rs` holds, in this order, `wide` citing `a/x` and `a/y` and `narrow` citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a moved test
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds only `one` and whose `tests/u.rs` holds `two`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a name two files share
- GIVEN base files whose `tests/t.rs` and `tests/u.rs` each hold a test `two` citing `a/x`, and head files in which only `tests/u.rs` holds it
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: a test that loses its marker
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds `one` and `two`, `two` with no marker
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: a title that carries its marker
- GIVEN base files whose `a.test.ts` holds a test titled `keeps the list (req:a/x@11111111)`, and head files in which the title ends `(req:a/z@22222222)`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: one of two tests of a name in one file
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, each citing `a/x`, and head files in which it holds one
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` once

#### Scenario: a cited test deleted beside an uncited test of its name
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, the first citing `a/x` and the second with no marker, and head files in which it holds one test titled `two`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` with `a/x`

#### Scenario: one of two tests of a name renamed
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, each citing `a/x`, and head files in which it holds one test titled `two` and one titled `second`, each citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a test that loses its last marker beside a new citing test
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds `one`, citing `a/x`, `two`, with no marker, and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: a cited test stripped beside a deleted uncited test of its name
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, the first citing `a/x` and the second with no marker, and head files in which it holds one test titled `two`, with no marker, and one titled `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` with `a/x`

#### Scenario: a cited test stripped beside a kept citing namesake
- GIVEN base files whose `a.test.ts` holds three tests titled `two`, the first two citing `a/x` and the third with no marker, and head files in which it holds two tests titled `two`, the first citing `a/x` and the second with no marker, and one titled `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` with `a/x` once

#### Scenario: uncited tests
- GIVEN files whose `tests/t.rs` holds a test `one` citing `a/x` and, at line 9, a test `helper_rounds_up` with no marker, and whose `a.test.ts` holds a `describe` group with no marker around a test that cites `a/y`
- WHEN `uncited_tests` runs
- THEN it returns `tests/t.rs::helper_rounds_up` with line 9, and nothing else

### Requirement: move-targets
`clm-coverage` SHALL offer `moved_to(graph, id)`, building without the `checkout` feature: the IDs of the requirements the spec holds that the moves `status-report` defines take the removed requirement or capability `id` to, followed as `progress-follows-moves` follows them, sorted, and nothing for an ID no live change removed or that no move takes anywhere.

#### Scenario: a moved requirement
- GIVEN an `applied` change that removed `g/x` and added `c/x`, and another that removed `g/y` and added nothing
- WHEN `moved_to` runs for `g/x`, for `g/y` and for `c/x`
- THEN it returns `c/x`, nothing and nothing

#### Serves
- capability/prove-by-tests

### Requirement: wait-lines
`clm-coverage` SHALL offer `waits(lines, line)`, building without the `checkout` feature: for `line`, one of the lines of `lines`, the whole list `pending` returned, the lines of `lines` it waits on, which are, for each ID in its `waits_on` in that order, every line of `lines` whose `id` is that ID, in the order `lines` holds them, other than `line` itself, a line being itself when both its `id` and its component are `line`'s. So an obsolete requirement's line on its own component, which waits on its own ID (`pending-report`), waits on that requirement's lines on the other components, and a wait on a requirement pending on the line's own component is returned like any other. It SHALL return nothing for a line whose `waits_on` is empty, and nothing for an ID no line of `lines` holds: it reads `lines` alone, so a caller that narrowed the list to one component passes the whole list, or gets no line of another component.

#### Scenario: an obsolete requirement's other lines
- GIVEN a root with `wiring: []`, components `b`, `c` and `d` with `paths` `["crates/b/**"]`, `["crates/c/**"]` and `["crates/d/**"]`, an `applied` change that removed `b/gone`, and markers citing `b/gone` in a file of each
- WHEN `waits` runs for `b/gone`'s line on `b`, with the whole list and with the list narrowed to `b`'s lines
- THEN the first returns its lines on `c` and `d`, in that order, and never its line on `b`, and the second returns nothing

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
