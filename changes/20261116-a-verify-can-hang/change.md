---
status: proposed
resolves: [a-verify-can-hang-on-a-test-until-the-plan-s-freeze-ends]
---

## Intent

The human makes `20261116-clm-verify-s-coverage` an exception to the plan's freeze (2026-11-16): `ci`'s next build builds `ci/tests-time-bounded`, `ci/coverage-script`, `ci/nextest-runner` and `ci/test-tools` now, not when the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` is done. The question item that held them back is resolved, and the exception is recorded where the freeze's other exceptions are, in the plan's item, and in the decision the requirements carry out, `one-runner-for-the-rust-tests`.

## Problem

`20261116-clm-verify-s-coverage` runs every Rust test run through nextest and bounds each test at 15 minutes. It is not one of the plan's phases, so its four requirements waited on a `question` item for the human to say whether to break the freeze. Meanwhile each verify that reaches the coverage pass under `cargo test` can hang without end, as one did for 100 minutes on 2026-11-16, and the mutation pass that runs it is phase 3's evidence; the release's `cargo test --all` would hang the same way.

## Approaches

- **Build it now, as an exception (the human's answer).** Taken. The plan's own evidence depends on verify finishing.
- **Wait for the plan's end.** Not taken: the human chose the exception.

## Outcome

- The question item is resolved, so it no longer blocks the four `ci` requirements, and `ci`'s next build builds them.
- `backlog/a-passing-check-proves-less-than-the-spec-claims.md` lists the exception beside the freeze's others, with the reason.
- `docs/decisions/one-runner-for-the-rust-tests.md` says it was built as an exception to the freeze, by the human, on 2026-11-16.
- No requirement, scenario or Design changes, so the change has no delta: the four requirements are already pending from `20261116-clm-verify-s-coverage`.

## Constraints and known edge cases

- `ci/npm-installs-bounded` still waits for the plan's end; it edits the same `coverage.sh` lines, so if the human later excepts it too, whichever `ci` build comes second rebases on the first.

## The human's answers

a-verify-can-hang-on-a-test-until-the-plan-s-freeze-ends: Build it now, as an exception — the human said: "answer the question to allow it to build now"

## Open questions

None.
