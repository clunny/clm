---
title: "A test whose braces don't balance on a line swallows every later test, and the mutation pass files false unproven items"
kind: bug
components: [check.coverage]
---

`clm-coverage`'s `tests_in` finds a Rust test's end by counting `{` and `}` outside a `//` comment, line by line. A char literal `'{'`, or a string holding a URL (`"https://..."`) on the same line as a JSON brace, leaves the count open, so the test's span runs to the end of the file and the scanner skips every `#[test]` inside it. `clm verify`'s mutation pass (`innermost`) then attributes every later marker to that one test, runs only it, and files a `kind: bug` item naming the requirements unproven although other tests prove them.

Seen in `crates/clm-guard/tests/guard.rs`: `the_hook_judges_with_the_hooks_budgets` (with `rest.find('{')`) swallowed lines 1165 to the end, and `backlog/unproven-plugin.hooks.guard-eb2b740e.md` named `block-refuses-whole-command`, `commands-in-crate`, `defect-approval` and `guard-per-agent-harness` as proven only by it, making all four `unbuilt`. Spot-check mutants show the real tests kill 27 of 28 mutants. The build of `plugin.hooks.guard` rewrote its tests to balance their braces; the scanner still misreads such tests elsewhere, silently.

Fix: skip string and char literals when counting (or at least `'{'`/`'}'` and `//` inside a string), or have the mutation pass refuse to judge a marker whose test span holds another `#[test]` attribute rather than filing it unproven.
