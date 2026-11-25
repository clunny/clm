---
status: done
resolves: [a-requirement-merged-into-one-of-another-slug-leaves-its]
---

## Intent

A change can record in `change.md`'s new key `moves` that a requirement it removes was merged into another of a different slug, on the same component or another. The recorded pair is a move like a same-slug move to another component: the removed requirement's obsolete markers wait on the requirement it merged into (`moved-markers-wait`), a change's progress follows it (`progress-follows-moves`), and `moved_to` returns it, so `check/obsolete-tests-elsewhere` and the build prompt's handling of another component's obsolete markers treat it as they treat a move already. (For a merge on the same component, the build prompt keeps its own wording, to re-cite each test to the requirement that now states what it proves, and doesn't name the target.) It answers `backlog/a-requirement-merged-into-one-of-another-slug-leaves-its`.

## Problem

`moves` (`crates/clm-coverage/src/status.rs`) counts as a move only a removal and an addition of the same slug on another component. Wave 6 (`20261124-wave-6-app-by-the-rule`) merged `web/live-binding`, `web/replica-binding` and `web/no-cron-trigger` into `web/worker-deploy`, whose build re-cites their tests. `worker-deploy` is blocked (Needs `web.worker/replica-class-export`), so the three obsolete lines can be neither re-cited, which would make a blocked requirement current out of order, nor deleted, since no `deleted-tests.txt` line names their tests. Yet `clm pending web` printed them unblocked, so the queue kept offering `web`, and three unattended builds in a row each stopped on the same three lines: the case `moved-markers-wait` was written to stop.

## Approaches

- **The change records the merge (chosen).** `moves: [{requirement: a/old, to: a/new}]` in `change.md`, checked by the model against the change's own deltas (the one removed, the other added or modified), and read by `check.coverage` into the one map of moves it already keeps. Only the change's author knows which requirement took a removed one's sentences, so the author says so, and a program checks that the pair is one the deltas make. Every reader of moves (`status-report`, `move-targets`, `pending-report`, `check/obsolete-tests-elsewhere`, `run`'s build prompt) gets it with no change of its own.
- **Infer the target.** Make an obsolete requirement wait on every blocked requirement its removing change adds or modifies on the same component. Rejected: it would hold back a plain removal, whose tests are listed for deletion, behind unrelated work, and it still can't tell a merge across components.
- **Annotate the removal in the delta** (`### Requirement: old <!-- base: … --> merged into new`). Rejected: the delta format is read by `apply`, the gate, `rename` and the Worker; a key in `change.md` is read by the model alone and sits beside `renames`, which records the same kind of fact.
- **Reuse `renames`.** Rejected: a rename is replayed mechanically by `clm rename` and changes no text, while a merge rewrites text, and `renames` refuses two entries giving one new name, which is what a merge of several requirements is.
- **Have the build file a `question` that holds the obsolete line** (`a-question-holds-a-removed-requirement-s-unlisted-test`). That is the workaround available today; it needs a build launched only to file it, and a human's answer for a fact the change already knew. Rejected as the fix.

## Outcome

- `model/parse-changes` accepts `moves` (and `parse_change_frontmatter` checks nothing of it, as with `renames`); one scenario added, *moves alone*.
- `model/move-records` (added): `moves` is a list of `{requirement, to}` mappings, both `<component>/<slug>`; on a live change the model reports as `model/move-records` an entry whose `requirement` the change's deltas don't remove, whose `to` they don't add or modify, a malformed entry or list, and a `requirement` named twice, and doesn't load the change. An archived change's `moves` loads unchecked.
- `check.coverage/status-report`: a change also moves `<a>/<x>` to `<b>/<y>` when its `moves` records it; scenario *merged later* added.
- `check.coverage/pending-report`: unchanged text; scenario *a merge into a requirement of the same component* added, showing the obsolete line blocked by its blocked target and released once the target's own blocker is current (the existing rule that leaves out a same-component blocker nothing blocks).
- `check.coverage`'s Design, **Moves**: names the two ways a move is made.
- Decision `merges-are-recorded-moves`, amending `progress-follows-moves`, `moved-markers-wait` and `a-question-holds-a-removed-requirement-s-unlisted-test` (a "Changed on" note on each).
- The glossary names the key (`clm/glossary-covers-terms`). `docs/spec-format.md`, which `clm guide spec-format` is built from, waits for the key to load (`checks-relax-before-use`): the propose skill's deferred item carries that docs edit.
- Two `kind: deferred` items, for the propose skill (`plugin`) and the draft prompt (`orchestrator.triage`), add the instruction to record a merge, once the key loads: prompt text, which no requirement states (`prompt-text-is-source`).

## Constraints and edge cases

- `model`'s build adds `moves` to `CHANGE_KEYS` (`model/term-lists`' test pins it to the frontmatter's fields) and re-exports the entry type beside `Rename`; `check.coverage`'s build rewrites the doc comment on `moves` in `crates/clm-coverage/src/status.rs`, which names only same-slug moves. Each is in its own component.
- `move-records` is checked after the change's deltas load, so a delta that fails to load also makes the entries naming its requirements report. An entry is reported once, by position, however many of its rules it breaks, as `renames` does. An entry whose `to` has the same slug on another component is accepted: it repeats the move the deltas make.

- One target per removed requirement. A requirement split into several of other slugs records one of them; a same-slug split across components is still followed to each as today.
- `to` must be added or modified by the same change, so its build is pending and is the one that re-cites the tests. A merge into a requirement the change leaves untouched isn't a merge.
- `clm rename` of a requirement a live change's `moves` names leaves that entry naming the old ID; the model then reports the change (`model/move-records`), so it shows, and the rename's author fixes the entry by hand. Not handled by `rename`: rare, and the report is the guard.
- No live change is edited: `20261124-wave-6-app-by-the-rule` can't record its three merges until `model/move-records` is built, since the model rejects an unknown key today.

## Open questions

- Wave 6's three `web` obsolete lines stay unblocked until either `web/worker-deploy` becomes current (its build re-cites them, ending the case) or `model/move-records` and `check.coverage/status-report` are built and a small spec branch adds `moves` to `20261124-wave-6-app-by-the-rule`. If `web` is offered for them again before then, a `question` item whose `blocks` names the three holds them today (`a-question-holds-a-removed-requirement-s-unlisted-test`); this change files none, since the human's wave-6 notes already tell `web`'s build what to do with them.
