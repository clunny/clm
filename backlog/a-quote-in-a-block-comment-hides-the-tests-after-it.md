---
title: "A quote in a block comment hides the tests after it"
kind: bug
components: [check.coverage]
---

`check.coverage/test-spans` (90fb3da6) has the span scanner read string literals across lines and leave block comments and regular expression literals unread. A quote in what it doesn't read therefore opens a literal that may never close:

- Rust: `/* a 12" ruler */` followed by `#[test] fn a() {}` and `#[test] fn b() {}` gives no tests at all: every later line reads as inside a string, so its test attributes start nothing and its markers count as `uncounted`. Before the literal reading, the same file gave both tests.
- TypeScript and JavaScript: a backtick in a regular expression or a block comment (`const r = /`/;`), or an odd number of backticks nested in a template's `${…}`, opens a template literal that runs across lines, so the call's span runs to the end of the file, swallowing every later test.

The component's Design promises that a misread costs one test's markers, never the tests after it; these cases break that. A fix needs the requirement to say more, so it is a spec change: for instance, read block comments too (`/* … */`, nesting in Rust), or end a Rust string that would cross a line holding a test attribute at the start of that line.

Found by the code review of the `impl/check.coverage` build that brought `test-spans` to 90fb3da6. This repository's own coverage reads the same before and after that build, so nothing here is hit yet.
