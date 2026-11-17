---
status: done
resolves: [an-unknown-component-in-clm-pending-names-a-rule-that-no-longer-states-it]
---

## Intent

`clm pending <component>` for a component the spec graph doesn't hold prints a violation whose rule is the requirement that states that refusal, `check/pending-narrows-on-a-graph`, not `check.coverage/pending-report`, which no longer says anything about unknown components. `check`'s build changes the string in `pending_for` and the tests that assert it.

## Problem

`20261117-clauses-only-a-command-shows` moved the narrowing of the pending list and its refusal of an unknown component from `check.coverage/pending-report` to `check/pending-narrows-on-a-graph`, but left `pending-narrows-on-a-graph` saying the violation carries the rule `check.coverage/pending-report`, and `check`'s Design saying the same. `crates/clm-check/src/lib.rs` follows that text, so `clm pending nope` names a requirement that says nothing about unknown components, and a reader who looks the rule up finds no refusal there.

The fix changes what a requirement states, so it is this change (`code-fix-needs-a-written-rule`). Nothing since 2026-11-10 fixes it: the only change touching the requirement is the one that left it so, and no commit with a `Resolves: backlog/` trailer since then touches `pending_for`.

## Approaches

- **Taken: the violation names `check/pending-narrows-on-a-graph`.** It is the requirement that states the refusal, so a violation's rule points at the text that explains it, as every other violation's does. One string and three test assertions change.
- **Not taken: moving the refusal back into `check.coverage/pending-report`.** That change moved it out on purpose, since only `clm-check`'s narrowing can show it; moving it back would undo that split.
- **Not taken: accepting it.** A violation whose rule says nothing about the violation misleads the one reader it exists for, and the fix is a string.

## Outcome

- `check` Design: `pending_in`'s unknown-node violation carries the rule `check/pending-narrows-on-a-graph`.
- `check/pending-narrows-on-a-graph` (modified): the violation for a component the graph doesn't hold is of the rule `check/pending-narrows-on-a-graph`; its scenario `a removed component is unknown` says both refusals carry that rule. Its build changes the `rule` in `pending_for` (`crates/clm-check/src/lib.rs`) and the assertions in `crates/clm-check/tests/pending.rs`, re-citing them at the new version.

## Constraints

- No edge cases or workarounds are known. Nothing outside `clm-check` reads the violation's rule: `view.repo/arguments-and-exit` prints whatever `pending_for` gives, and no capability test asserts it.
