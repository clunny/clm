---
component: check.coverage
---

## MODIFIED Requirements

### Requirement: test-spans <!-- base: 2ed7e7724ea12c28 -->
`clm-coverage` SHALL offer `tests_in(path, text)`, returning the tests and the named groups of tests in one file's text, in the order they start, each with its name, whether it is a group, and its first and last line, the first being the first of the comment and attribute lines directly above it with no blank or code line between, and SHALL find them by the file's extension:
- `.rs`: a test is a function that an attribute precedes whose path's last segment is `test`, `rstest` or `test_case` or ends `_test` (`#[test]`, `#[tokio::test]`, `#[wasm_bindgen_test]`), named by the function, from those lines to the line that closes its body, found by counting braces outside line comments; there are no groups. A comment line between the attribute and the function is one of its lines.
- `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` and `.cjs`: a test is a call of `it` or `test`, and a group a call of `describe` or `test.describe`, each with or without the modifiers `check/skipped-tests` reads and `.only`, `.each`, `.concurrent` and `.serial`, starting a line after leading whitespace, named by its first argument when that is a string literal and by an empty name otherwise, from those lines to the line that closes the call, found by counting its parentheses outside line comments.
- `.py`: a test is a function, `def` or `async def`, whose name starts `test`, and a group a class whose name starts `Test`, named by it alone, from the decorator and comment lines directly above it to the last line indented deeper than its `def` or `class` line.
- any other extension: none.

It SHALL offer `markers_outside_tests(path, text)`, returning each marker `marker_spans` finds in `text` on a line that belongs to no test or group, as `Marker`s, and nothing for a Markdown file or a file of another extension than those above. `scan(root)` SHALL return them for every file it reads as `outside`, beside `req`. Both calls SHALL build without the `checkout` feature. Which markers `markers_in` returns and `scan`'s `req` holds is `markers-in-tests`'s to say, not this requirement's: `outside` holds the markers outside tests whether or not `req` holds them too.

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
