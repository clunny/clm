---
component: check.coverage
base: { contract: "06efee2aad7167fa", design: "7005cc2ccf999d1e" }
---

## MODIFIED Contract

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
- `uncounted(graph, outside)`, `obsolete_tests(graph, files)` and `deleted_tests(base_files, head_files)`: the markers outside tests by what they cite, the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`); `clm coverage` prints them through `clm-check` (`check/coverage-lists`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`); `check.affected` reads it through `clm-check` (`check/path-ownership`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## MODIFIED Design

Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside line comments. That is approximate on purpose: a brace inside a string can end a span early or late, which costs one marker its standing and shows in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never a crash. In a file of any other extension CLM can't tell a test, so every marker counts as before; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.


## ADDED Requirements

### Requirement: test-spans
`clm-coverage` SHALL offer `tests_in(path, text)`, returning the tests and the named groups of tests in one file's text, in the order they start, each with its name, whether it is a group, and its first and last line, the first being the first of the comment and attribute lines directly above it with no blank or code line between, and SHALL find them by the file's extension:
- `.rs`: a test is a function that an attribute precedes whose path's last segment is `test`, `rstest` or `test_case` or ends `_test` (`#[test]`, `#[tokio::test]`, `#[wasm_bindgen_test]`), named by the function, from those lines to the line that closes its body, found by counting braces outside line comments; there are no groups. A comment line between the attribute and the function is one of its lines.
- `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs`: a test is a call of `it` or `test`, and a group a call of `describe` or `test.describe`, each with or without the modifiers `check/skipped-tests` reads and `.only`, `.each`, `.concurrent` and `.serial`, starting a line after leading whitespace, named by its first argument when that is a string literal and by an empty name otherwise, from those lines to the line that closes the call, found by counting its parentheses outside line comments.
- `.py`: a test is a function, `def` or `async def`, whose name starts `test`, and a group a class whose name starts `Test`, named by it alone, from the decorator and comment lines directly above it to the last line indented deeper than its `def` or `class` line.
- any other extension: none.

It SHALL offer `markers_outside_tests(path, text)`, returning each marker `marker_spans` finds in `text` on a line that belongs to no test or group, as `Marker`s, and nothing for a Markdown file or a file of another extension than those above. `scan(root)` SHALL return them for every file it reads as `outside`, beside `req`. Both calls SHALL build without the `checkout` feature. Neither changes what `markers_in` returns or what `scan`'s `req` holds.

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
`markers_in(path, text)` and `scan(root)`'s `req` SHALL hold, for a file of an extension `test-spans` reads, only the markers on a line of a test or a group of tests, and for any other file every marker, none for Markdown, so a marker in code that isn't a test proves nothing (`markers-count-in-tests`). The capability test `crates/clm/tests/capability/prove_by_tests.rs` writes its marker inside a test function; this requirement's build adds to it that the same marker in a function with no test attribute leaves the requirement `unbuilt`.

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
`clm-coverage` SHALL offer, building without the `checkout` feature and taking files as paths and texts: `uncounted(graph, outside)`, returning, sorted by ID, each requirement or capability that exists, or that a change outside the archive removed (`removed`), and that a marker outside a test cites, with those markers' locations; `obsolete_tests(graph, files)`, returning each test (`test-spans`, groups left out) that holds a marker citing a requirement or capability a change outside the archive removed, sorted by file and then by line, with the removed IDs it cites, sorted, and whether every marker on the test's lines cites one; and `deleted_tests(base_files, head_files)`, returning each test that holds a marker in `base_files` and that `head_files` holds no test of that name in a file of that path for, sorted the same way, with the IDs it cited. A test is named `<file>::<name>`.

#### Scenario: uncounted
- GIVEN `a/x` existing, `a/gone` removed by an `applied` change, and markers outside a test citing `a/x` at `src/lib.rs:3` and `a/gone` at `src/lib.rs:9`
- WHEN `uncounted` runs
- THEN it returns `a/gone` with `src/lib.rs:9`, then `a/x` with `src/lib.rs:3`

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `only_x` citing `a/x` alone and a test `both` citing `a/x` and `a/y`, both in `tests/t.rs`, `both` first
- WHEN `obsolete_tests` runs
- THEN it returns `tests/t.rs::both` with `a/x`, not every marker, then `tests/t.rs::only_x` with `a/x`, every marker

#### Scenario: a deleted test
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and head files whose `tests/t.rs` holds only `one`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: a renamed test
- GIVEN the same, with `two` renamed `second` at the head
- WHEN it runs
- THEN it returns `tests/t.rs::two`, since a program can't tell a rename from a removal

## MODIFIED Requirements

### Requirement: coverage-report <!-- base: 1fec41ce77dadfbc -->
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

### Requirement: marker-spans <!-- base: 3ee7977dc0e1f7ed -->
`clm-coverage` SHALL offer `marker_spans(text)`, returning each `req:` marker in `text`, inside a test or not, in the order they appear, with the ID and version `markers_in` and `markers_outside_tests` give it, with the byte range of its whole text (`req:` through its slug, and through its version when it carries one), so a caller that rewrites markers in place (`rename/marker-rewrite`) rewrites every marker, the ones the coverage counts and the ones it reports as `uncounted`. It SHALL build without the `checkout` feature.

#### Scenario: two markers on a line
- GIVEN the text `// req:a/x@11111111 and req:b/y`
- WHEN `marker_spans` runs
- THEN it returns `a/x` at version `11111111` spanning bytes `3..19`, and `b/y` with no version spanning bytes `24..31`, each range end-exclusive

#### Scenario: every marker of the file
- GIVEN the text of `t.rs` with a marker in a test, a marker in a function that is no test, a removal marker and a `req:` that follows a word character
- WHEN `marker_spans`, `markers_in` and `markers_outside_tests` run on it
- THEN `marker_spans` finds the two markers in order, the first the one `markers_in` returns and the second the one `markers_outside_tests` returns
