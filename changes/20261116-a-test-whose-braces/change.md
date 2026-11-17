---
status: applied
resolves: [a-test-whose-braces-don-t-balance-on-a-line-swallows-every-later]
---

## Intent

`clm-coverage`'s test scanner stops misreading a test that names a brace or a parenthesis in a string or character literal, and a Rust test it still misreads can no longer swallow the tests after it. `check.coverage/test-spans` counts braces and parentheses outside string and character literals as well as line comments, and ends a Rust test's span no later than the line above the next test's first line. With spans right, the mutation pass stops running one test for every later marker in a file and filing their requirements unproven.

## Problem

`tests_in` finds a Rust test's end by counting `{` and `}` outside `//` comments. A character literal `'{'`, or a string holding a URL whose `//` reads as a comment beside a brace, leaves the count open, so the span runs to the end of the file, the scanner skips every `#[test]` after it, and `clm verify`'s mutation pass, which judges each marker by the innermost span holding it, ran that one test for every later marker. In `crates/clm-guard/tests/guard.rs`, `the_hook_judges_with_the_hooks_budgets` (with `rest.find('{')`) swallowed every later test, and the pass filed four `plugin.hooks.guard` requirements unproven that other tests prove, making them `unbuilt`.

The Design accepted the miscount on purpose, as costing "one marker its standing": it costs every later test's markers, and through the mutation pass false `kind: bug` items. So the design is wrong, not only the code, and the requirement's text, "counting braces outside line comments", states the behaviour that misreads. The fix is a requirement change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-09 fixes it: `20261114-test-spans-leaves-counting` only moved a sentence about `markers_in`, and the guard's build rewrote its own tests to balance their braces, leaving the scanner as it was.

## Approaches

- **Taken: skip literals when counting, and bound a Rust test's span by the next test.** Skipping literals fixes the cases seen, which are common in tests (`find('{')`, JSON in strings, URLs). The bound limits what is still misread (a block comment, a regular expression literal) to one test, which `clm coverage`'s `uncounted` line shows. Both are a few lines of the existing line scanner: no parser, so `clm-coverage` still builds for `wasm32-unknown-unknown`.
- **Rejected: only have the mutation pass refuse to judge a marker whose test span holds another test attribute.** It stops the false items but leaves the spans wrong, so `markers_in` and `markers_outside_tests` still count those markers wrongly for `clm coverage` and the Worker; and it moves a scanner fault into `verify`.
- **Rejected: a real parser per language.** The Design already rejects it: a parser generator's C runtime doesn't build for the Worker's target, and the scanner needs only where a test starts and ends.

## Outcome

- `check.coverage/test-spans`: Rust counts braces outside line comments and string and character literals, and a test's span ends no later than the line above the next test's first line; TypeScript and JavaScript count parentheses outside line comments and string and template literals. Three scenarios: braces in literals, a brace the scanner doesn't read, a parenthesis in a string.
- `check.coverage`'s Design names what the scanner reads to skip a literal, what it still misreads, and why the bound matters to the mutation pass.

## Edge cases and workarounds

- A Rust character literal is told from a lifetime or label (`'a`) by its closing quote: `'x'`, `'\x'`, `'\u{..}'` are literals, `'a` followed by anything but `'` is not.
- Rust strings and template literals may run across lines, so the literal state carries from line to line.
- A template literal's `${…}` is read as part of the string; a parenthesis inside it is skipped, which leaves the count right whenever the expression's own parentheses balance.
- A Rust file is read for literals from its first line, so a fixture string holding `#[test]` (as `crates/clm-coverage/tests/test_spans.rs` has) neither starts a test nor bounds the one holding it. The bound is the next test's first line as `tests_in` returns it, the comments above its attribute included.
- A TypeScript or JavaScript `'` or `"` string ends at its line's end, so a stray apostrophe (a regular expression, JSX text) costs one line.
- Block comments and regular expression literals are not read; the Rust bound covers them, and TypeScript and JavaScript stay approximate for them.

## Constraints

No file, key or format an adopter holds changes. `clm-coverage` stays a line scanner that builds without the `checkout` feature and for `wasm32-unknown-unknown` (`coverage-builds-for-wasm`).
