---
component: check.coverage
base: { design: "b4c532e76ef8849b" }
---

## MODIFIED Design
Crate `clm-coverage` holds the modules `coverage`, `pending`, `status` and `glob`; `clm-check` depends on it, and the `clm` commands (`clm coverage`, `clm pending`, `clm status`) reach it through `clm-check`'s checkout wrappers (`check-coverage-split`). A violation that enforces one of its requirements carries the rule `check.coverage/<slug>`, wherever its code sits. The default feature `checkout` holds what reads a checkout (`scan`, through `git`).

**Markers.** `scan` reads the files `git ls-files --cached --others --exclude-standard` lists, skipping Markdown (`*.md`, where markers are examples rather than claims), for markers matching `req:<node-id>/<slug>`, where `req:` doesn't follow a word character, followed by `@` and 8 lowercase hex digits when the marker carries a version, and for removal markers matching `clm:remove-by (\d{8}-[a-z0-9-]+)`. Outside a git work tree, such as a tree `clm wrap-up` extracts, `scan` walks every file under the root instead, as `clm-check`'s scan does. `markers_in` applies the same reading to one file's text, so a caller with files in memory, such as the Worker, gets what `scan` would (`markers-from-files`).

**Pending** (`pending-report`). The list carries every blocker a requirement has, items in `blocked_by` and everything else in `waits_on`, so the queue needs nothing else. An obsolete requirement a move took elsewhere waits on the move's target because the build that re-cites the moved requirement re-points its markers, so the old component has nothing to do with them until that build lands (`moved-markers-wait`). A pending capability waits on the rules that serve it because its test exercises them; a `behind` rule that nothing blocks doesn't hold it, so an edit to an old rule that can be built now leaves an existing capability's test free to follow its own edit (`serving-rules-order-capabilities`). The serving rules are found by one pass over the graph's requirements per call, as a map from each capability to the rules that serve it, made after each rule's own blockers are known: a rule is never an entry and a `Needs` can't name one, so a rule's blockers never include a capability's wait, and the capabilities' are computed from the rules', with no fixed point to reach. This is build order, not a `#### Needs`, though the queue seeds the capability's priority through it as through a need (`orchestrator/priority-orders-components`): the model's rules for needs (no need on an optional component from a required one, no cycle) don't apply, and it can't deadlock, since nothing waits on a capability in turn.

**Progress** (`change-progress`). `change_progress` computes the coverage once, and `status_report`'s change rows, `complete_changes` and `view`'s `flight` model all take a change's counts from it, so the rule lives in one place and `clm status`, `clm rebase`, `clm wrap-up` and `clm view flight` can't disagree about a change. It returns every change outside the archive whatever its status; a caller that shows only some statuses filters them.

**Moves** (`progress-follows-moves`). Moves are what `status-report` defines. Only `ADDED Requirements` make a move, so neither delta needs `create: true` or `remove: true`, and a requirement moved between two existing components is a move too. A move's added text needn't match the removed text, and moves are not ordered against the change they're followed for: a rename may apply after a change whose deltas target the old component. The moves are computed once per call into a map from each moved ID to the IDs it moved to, and an ID is followed through it breadth first, stopping at the IDs the spec holds and visiting each ID once, so a chain of renames is followed and one that comes back is not followed forever. An ID that moved to more than one component (a split) counts as each of them, so a split can make `total` larger than the number of requirements the deltas list; the counted IDs are deduplicated after the mapping. `pending` reads the same map, through the same walk, for the requirements an obsolete one waits on; `report` and `removed` ignore the moves.

**Tests** (`test-spans`, `markers-in-tests`, `markers-count-in-tests`). A marker proves something only where a test can run it, so the scan reads where the tests are. It reads three language families by their file extension, the ones `check/skipped-tests` reads, with a line scanner and no parser: a test's lines run from the comment and attribute lines directly above it to the line that closes its body, found by counting braces (Rust), the call's parentheses (TypeScript and JavaScript) or indentation (Python) outside comments and string and character literals, since a test that names a brace, as `find('{')` or a JSON string does, or a URL whose `//` would read as a comment, is common and was misread. The scanner reads only what it must to skip a literal or a comment: a block comment, `/* … */`, running across lines and nesting in Rust, since a quote in one, as `/* a 12" ruler */`, would otherwise open a string that never closes; Rust's `"…"` and raw `r#"…"#` strings, with any `b` or `c` prefix and running across lines, and a character literal, `'{'` or `'\''`, told from a lifetime or label (`'a`) by its closing quote; and the three quotes of TypeScript and JavaScript, a template literal's `${…}` read as part of the string and a `'` or `"` string ending at its line's end, so an apostrophe in a regular expression or JSX text costs one line, not the file. A Rust file is read for literals and block comments from its first line, so a fixture string or a commented-out test holding `#[test]` neither starts a test nor bounds the one holding it. Rust's literals and comments are then read whole, but what TypeScript and JavaScript don't read, a regular expression literal holding a quote or a parenthesis, or a template literal nested in a `${…}`, and a brace a line scanner still miscounts in either, can leave a span open, so a test's span also ends no later than the line above the first line of the next test (or, in TypeScript and JavaScript, group): a misread costs that one test's markers, which show in `clm coverage`'s `uncounted` line (`check/coverage-lists`), never the tests after it and never a crash. The bound matters beyond the count: the mutation pass judges each marker by the innermost test holding it (`verify/mutation-pass`), so a span that ran to the end of its file had it run one test for every later marker and file their requirements unproven, though other tests proved them. A group takes no such bound, since its call holds its tests: a misread in a group's own code, outside its tests (a hook matching ``/`/``), can still run the group's span to the end of its file, which leaves its tests' spans and the innermost test of every marker as they are and costs only markers outside any test after it, counted as the group's; reading regular expressions to close that gap would mean telling one from a division and from JSX's `</`, much code for a rare case. Python's spans end by indentation and read no literal. In a file of any other extension CLM can't tell a test, so every marker counts as before; a language is added to the list by a change. Whether a test passed is not read from any runner: every file a branch changes is in the selection of the check behind its land's verdict (`land/land`, `check.affected/affected-files`), so the test that holds a new or re-cited marker ran in a check that passed, and what a check never ran shows after the land, when the mutation pass finds its tests kill nothing (`verify/mutation-pass`). It is a scanner and not three parsers because `clm-coverage` builds for `wasm32-unknown-unknown` for the Worker, which a parser generator's C runtime doesn't, and because it needs only where a test starts and ends, which a grammar for each language would be much code to learn. `markers_in` stays a function of one file's path and text, so the Worker and `clm view --at` count what the CLI counts with no ledger (`markers-from-files`).

**Who reads which markers.** What asks whether a requirement is proven reads the markers that count: the coverage, `clm pending`, the size count, `check/tests-cite-their-owner`, the churn signal and the views. What asks which text names a requirement reads every marker, `scan`'s `outside` included: `check/dangling-markers`, the rename's rewrite (`marker_spans`) and test selection (`check.affected/affected-files`), which must still run a file for the component it cites when its marker sits outside a test.

**Unproven** (`coverage-report`). A requirement that a backlog item's `unproven` names at its current version reads `unbuilt` whatever cites it, so a verdict of the mutation pass reaches every reader through the committed tree, and the item's deletion by the build that mends the test brings the requirement back.

## MODIFIED Requirements

### Requirement: test-spans <!-- base: 90fb3da6eb3d1c82 -->
`clm-coverage` SHALL offer `tests_in(path, text)`, returning the tests and the named groups of tests in one file's text, in the order they start, each with its name, whether it is a group, and its first and last line, the first being the first of the comment and attribute lines directly above it with no blank or code line between, and SHALL find them by the file's extension:
- `.rs`: a test is a function that an attribute precedes whose path's last segment is `test`, `rstest` or `test_case` or ends `_test` (`#[test]`, `#[tokio::test]`, `#[wasm_bindgen_test]`), named by the function, from those lines to the line that closes its body, found by counting braces outside line comments, block comments (`/* … */`, nesting) and string and character literals, and ending no later than the line above the next test's first line, the end of the file when no test follows; the reading of literals and block comments carries across lines, so a test attribute inside a string or a block comment starts no test; there are no groups. A comment line between the attribute and the function is one of its lines.
- `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs`: a test is a call of `it` or `test`, and a group a call of `describe` or `test.describe`, each with or without the modifiers `check/skipped-tests` reads and `.only`, `.each`, `.concurrent` and `.serial`, starting a line after leading whitespace, named by its first argument when that is a string literal and by an empty name otherwise, from those lines to the line that closes the call, found by counting its parentheses outside line comments, block comments (`/* … */`) and string and template literals, a `'` or `"` string ending at the end of its line and a block comment and a template literal running across lines; a test's span, not a group's, ends no later than the line above the first line of the next test or group, the end of the file when none follows. Each call's reading starts on its own line, so a call starts a test or a group whatever literal or block comment a line above it left open.
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
