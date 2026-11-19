---
status: applied
resolves: [pending-cannot-move-an-obsolete-marker-to-its-file-s-owner-while-clm-run-s-tests-expect-it-on-its-own-component, an-item-cannot-hold-a-requirement-a-done-change-removed, clm-run-lists-an-obsolete-requirement-s-foreign-markers-no-rule-states]
---

## Intent

Let `clm pending` place an obsolete requirement's marker on its file's owner, which `20261119-a-removed-requirement-s-tests-elsewhere` and `20261119-an-obsolete-requirement-nobody` designed and whose first build was reverted. It settles the two questions those builds filed, as the human agreed on 2026-11-19, and finishes step 2 of phase 5 (`backlog/a-passing-check-proves-less-than-the-spec-claims`), so it is the plan's work.

## Problem

`check.coverage`'s build of the placement failed its land on three tests of `crates/clm-run/tests/run.rs`, which assert the old placement through the real `pending` and which that build may not edit. The change had ordered `orchestrator.run` first to prevent it. That build ran first and did the reverse of its step, for two reasons:

- the step was written under "What the builds do" in `change.md`, which no build is given: a build's prompt holds requirement diffs and its context bundle;
- the step asked it to remove the path for the old placement while `pending` still gave the old placement, so a prompt would have told a build to delete a test in a file the land refuses it. The build kept the code and filed `backlog/clm-run-lists-an-obsolete-requirement-s-foreign-markers-no-rule-states`.

Reviews of the reverted build found four more faults, all in the design:

- the queue's priorities take a waited requirement's component from the first part of its ID, so a line that waits on its own ID seeds its own component;
- the blockers view keys the pending list by ID, which folds a requirement's lines into one;
- the build prompt would say `b/gone` is blocked by `b/gone`;
- an obsolete line drops a move's target on its own component whatever the target's state, so with the target blocked the build is told to re-cite a test to a requirement its prompt left out.

A review of `model`'s build asked whether an item may block a requirement that a `done` change outside the archive removed.

## Approaches

- **Taken: three steps** (`expand-contract`). `orchestrator.run` reads both placements, which its code does today; `check.coverage` switches; `orchestrator.run` removes the old path. `check.coverage/pending-report` and `removed-capabilities` lose their need of `orchestrator.run/pending-prompt`, which now needs `pending-report`.
- **Taken: a small code branch takes the three assertions out of `run.rs` before `check.coverage` builds.** A requirement names no test, and the step states no behaviour. The same branch marks the old path `clm:remove-by` this change, so the change can't be archived while the path stays.
- **Taken: the own ID stays in `waits_on`, and `clm-coverage` offers `waits`, which resolves a wait to lines** (`check.coverage/wait-lines`, decision `a-wait-names-a-line`). The three readers use it.
- **Taken: a move's target on the line's own component stays a blocker while it is itself blocked.**
- **Taken: `done` stays out of `model/parse-backlog`.** `check/archive-done` already fails a `done` change outside the archive, so no tree that passes `clm check` holds the case. The decision `a-question-holds-a-removed-requirement-s-unlisted-test` says so; no requirement changes.
- **Not taken: a field of its own for the wait.** A third kind of blocker in the queue's JSON, the web app and the panel, and a struct `clm-run` builds by literal.
- **Not taken: removing the old path first.** The prompt would be wrong for the case between two lands, and the build that was asked to declined.

## Outcome

- `check.coverage/wait-lines` (new): `waits(lines, line)`, the lines a line waits on.
- `check.coverage/pending-report`: a blocked move target stays on its own component's line; the need of `orchestrator.run/pending-prompt` goes. `removed-capabilities` loses the same need.
- `orchestrator/priority-orders-components`: a wait seeds the component of each line `waits` gives.
- `view/blockers-view`: follows every wait through `waits`.
- `orchestrator.run/pending-prompt`: names the components a line that waits on its own ID waits for, and lists an obsolete requirement of its own with no marker of another component's file.
- `deleted-tests.txt`: one test, which proves only the old path.

## Constraints

- A project that sets no `wiring` sees no change, and `wiring` is in no release.
- No file, key or format a released project holds changes.

## What the requirements no longer state

Nothing: no requirement, scenario or sentence holding `SHALL` goes. The one cut is the test `crates/clm-run/tests/prompt.rs::an_obsolete_test_in_another_components_file_is_listed_apart`. It proves the listing apart of an obsolete marker, which `orchestrator.run/pending-prompt` stopped stating on 2026-11-19 with the human's yes, and which `pending` no longer leads to once `check.coverage` is built. Its namesake in `run.rs` is kept and renamed by the small branch, for the two cases it shows that stay true.

## The order of the builds

1. The small code branch, by the session that proposed this, once this change is on `main`.
2. `check.coverage`: `wait-lines`, `pending-report`, `removed-capabilities`.
3. `orchestrator.run`, `orchestrator` and `view`, each needing step 2.

Each step that a build takes is stated in its requirement or in `backlog/clm-run-lists-an-obsolete-requirement-s-foreign-markers-no-rule-states`, which names the code and the test `orchestrator.run`'s build removes.
