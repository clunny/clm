---
status: done
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its last change: a build changes only its own component's files. `path-ownership` let a build change any file that cites its component, a clause written for the tests Cargo kept in the root package's directory. `20261115-commands-in-their-crates` moves those tests into their components, so the clause goes: `clm land` refuses a build that changes another component's file whatever the file cites.

It edits `ARCHITECTURE.md`, so it needs review. It removes one requirement and one scenario's outcome, listed under What it removes.

## Problem

`own-files-at-the-gate` lets a build's branch change a file whose text holds `req:<component>/`. It opens the whole file, to add, edit or delete, and 180 of 429 builds used it. It was the only way a component could keep its proof while 872 of its tests sat in the root's files and 111 markers in 18 files of a parent or a sibling. While it stands, two builds can edit one test file, a build can break another component's tests, and the guide carries an exception that the open question "may a build remove a test in a file another component owns" kept reopening.

## Approaches

- **Remove the clause once the tests have moved, ordered by `#### Needs` and `blocks`.** Taken. `gate/own-files` needs `clm/routes-only`, which needs all 23 `commands-in-crate` requirements, and three backlog items block it. The spec says so today and the queue builds it last.
- **Remove it now and exempt the files that still need it.** Not taken: about 56 exemptions, each a reviewed line that a later change removes.
- **Keep the clause for files outside any crate.** Not taken: a rule with a standing exception is the shape `redesign-before-workaround` asks to end.

## Outcome

- **`gate/own-files`** no longer lists a file that cites the component among those a build may change. It needs `clm/routes-only`.
- **`land/build-holds-own-files`** prints a line that names three kinds of file, not four.
- **`ARCHITECTURE.md`, `path-ownership`**: "What else a build may change, the wiring and the tests that cite it, is in the guide" becomes "What else a build may change is in the guide". The list is mechanics and the guide holds it (`human-layer-entries-stay-short`).
- **The guide is not edited here.** `clm guide how-it-works` still lists the exception, which `clm land` grants until `gate`'s build; the guide loses it on the spec branch that follows that build (`backlog/nothing-holds-the-root-s-test-files-to-the-root-s-requirements`), so the guide a binary prints matches what it does.
- **`gate/binary-tests-apart` is removed**, and `gate`'s Design says its tests are all its crate's.
- **The backlog items** that move the remaining tests block `gate/own-files`: `tests-of-components-with-no-crate-sit-in-the-root-s-test-files`, `some-tests-prove-a-component-from-another-component-s-files` and `web-tests-prove-one-web-component-from-another-s-files`.
- **`backlog/a-build-is-told-to-re-cite-a-marker-in-a-file-it-may-not-change`** blocks `gate/own-files` too: `orchestrator.run/pending-prompt` still tells a build to re-cite a `behind` marker in another component's file, which the land will refuse. It needs a spec change of its own once the tests have moved.
- **`backlog/the-proof-is-real-upgrade-note-says-a-foreign-change-needs-review`**: a bug the builder check found in an unreleased upgrade note, filed and not fixed.
- **`backlog/nothing-holds-the-root-s-test-files-to-the-root-s-requirements`**: afterwards, the root's files get a `tests` glob so the existing `check/tests-cite-their-owner` judges them, and two instructions lose the sentence that described the clause.

On the same branch: `20261115-one-route-to-coverage` (archived there) and its contract step `20261115-check-passes-on-no-coverage`, left proposed.

## What it removes

- **One scenario's outcome.** `gate/own-files`, scenario "a test that cites the component": a build that adds a test to another component's file, or deletes one there, was let through, and is now refused with that component named. This is the behaviour the phase exists to end.
- **The requirement `gate/binary-tests-apart`**: every `gate` test that runs the binary sits in `crates/clm/tests/gate.rs` and no other root test file cites `gate`. It states where a test file sits, not what the product does; `gate/commands-in-crate` puts those tests in `gate`'s crate, and its one test, a scan of the root's test files, is deleted by `gate`'s build.
- **With `20261115-check-passes-on-no-coverage`, when it is applied:** the requirement `check/path-ownership`. What it states of `owner` and of matching one glob moves to `check.coverage/owner` with both scenarios; the name `path_matches`, and three sentences that state no behaviour, go.

No test is dropped by this change.

## Constraints

- **Every land leaves `main` green.** The clause stays in the code until `gate`'s build of this requirement, which the queue holds until the moves are built and the three items are gone.
- **Adopters.** Neither `wiring` nor the clause has been in a release, so no project holds anything this changes and there is no migration. After the next release, a project that names `wiring` while its tests share files has its builds refused until it gives the tests to their components; the guide will say so when it loses the exception.
- **`clm land` gains no check.** The rule reads one clause fewer.

## Open questions

- From the architecture audit, left: `20261115-one-route-to-coverage` orders the twelve import moves by a `deferred` item, not by a requirement each as `expand-contract` words it. `check-coverage-split` records why: an import path states no behaviour.
- `gate/own-files` waits on `backlog/web-tests-prove-one-web-component-from-another-s-files`, which needs a design nobody has made. Until it is proposed, the clause stays in the code.
- A fix under `clm claim <component> --fix` to a test kept under another component's `paths` is refused once the clause is gone. By then no such test should exist; one that does is moved by its owner's build first.
