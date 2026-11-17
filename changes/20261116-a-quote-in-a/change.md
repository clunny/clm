---
status: applied
resolves: [a-quote-in-a-block-comment-hides-the-tests-after-it]
---

## Intent

`clm-coverage`'s test scanner reads block comments, so a quote in one no longer opens a string that hides every later test, and a TypeScript or JavaScript test's span is bounded by the next test as a Rust test's is, so what the scanner still misreads there costs that one test. `check.coverage/test-spans` counts braces and parentheses outside block comments too (`/* … */`, nesting in Rust), and ends a TypeScript or JavaScript test's span, not a group's, no later than the line above the next test's or group's first line.

## Problem

`20261116-a-test-whose-braces` made `test-spans` read string literals across lines and left block comments and regular expression literals unread, trusting a Rust test's bound to cap a misread at one test. A quote in what it doesn't read opens a literal that never closes, which the bound can't cap: the literal state runs on, so every later test attribute reads as inside a string and starts nothing.

- Rust: `/* a 12" ruler */` above two tests gives no tests at all, and their markers count as `uncounted`.
- TypeScript and JavaScript: a backtick in a regular expression or a block comment (`const r = /`/;`), or an odd number of backticks nested in a template's `${…}`, opens a template literal that runs to the end of the file, and the call holding it, with no bound, swallows every later test.

The Design promises a misread costs one test's markers, never the tests after it; these cases break it, and no requirement states the behaviour that keeps it, so it is a spec change (`code-fix-needs-a-written-rule`). No change or code fix since 2026-11-09 touches it: the item was filed from the review of the build that made `test-spans` 90fb3da6, and `20261115-check-passes-on-no-coverage` leaves the scanner alone.

## Approaches

- **Taken: read block comments in both language families, and bound a TypeScript or JavaScript test's span by the next test or group.** In Rust a quote outside a comment is always a literal's, so reading `/* … */` (nesting) makes Rust's reading whole: nothing left there can open a string that never closes. TypeScript and JavaScript keep two things unread, a regular expression literal and a template nested in `${…}`; the bound caps either at one test, as the Rust bound already caps a miscounted brace. A few lines each in the existing line scanner; no parser, so `clm-coverage` still builds for `wasm32-unknown-unknown`.
- **Rejected: end a Rust string at a line holding a test attribute at its start.** The item's other suggestion. It breaks what the Design promises a fixture string holding `#[test]` (`crates/clm-coverage/tests/test_spans.rs` has several), and it is a guess where reading the comment is exact.
- **Rejected: read TypeScript and JavaScript regular expression literals.** Telling one from a division and from JSX's `</` or `/>` is a heuristic of its own whose misjudgement blanks a line's closing `)`, much code for a rare case the bound already caps for tests.
- **Rejected: accept it.** A block comment holding a quote is ordinary Rust (a doc-like comment, a measurement), and the cost is every later test of the file: the false `unproven` items the mutation pass filed before `20261116-a-test-whose-braces` came back.

## Outcome

- `check.coverage/test-spans`: Rust counts braces outside block comments too, nesting, read across lines, so a test attribute inside one starts no test; TypeScript and JavaScript count parentheses outside block comments too, and a test's span, not a group's, ends no later than the line above the next test's or group's first line. The scenario `a brace the scanner doesn't read` becomes `a brace left open`, since a brace in a block comment is now read; three scenarios are added: `a quote in a block comment`, `a backtick in a block comment` and `a backtick the scanner doesn't read`.
- `check.coverage`'s Design names block comments among what the scanner reads, what it still misreads, the bound for TypeScript and JavaScript tests, and the gap a group keeps.

## Edge cases and workarounds

- A Rust block comment nests (`/* /* */ */`); a TypeScript or JavaScript one doesn't.
- A `/*` or `*/` inside a string or a character literal is the literal's, and a `"` inside a block comment is the comment's; `//` inside a block comment starts no line comment.
- A line inside a block comment that starts with `/*` or `*` is a comment line, so a block comment directly above a test is among its first lines, as the scenario `a quote in a block comment` shows.
- A group's span takes no bound, so a misread in a group's own code (a hook holding ``/`/``) can still run the group to the end of its file; its tests' spans stay right, so only markers outside any test after it count, as the group's. Accepted: rare, and closing it means reading regular expressions.
- A `/*` inside a TypeScript or JavaScript regular expression (`/[/*]/`) opens a block comment; inside a test the bound caps it.

## Constraints

No file, key or format an adopter holds changes. `clm-coverage` stays a line scanner that builds without the `checkout` feature and for `wasm32-unknown-unknown` (`coverage-builds-for-wasm`).
