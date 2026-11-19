---
title: "Pending can't move an obsolete marker to its file's owner while clm-run's tests expect it on its own component"
kind: question
components: [check.coverage, orchestrator.run]
blocks: [check.coverage/pending-report, check.coverage/removed-capabilities]
---

Found by the build of `check.coverage` on 2026-11-19. Building `check.coverage/pending-report@ebf1b50f` and `check.coverage/removed-capabilities@d9e27527` means changing where `pending` puts an obsolete requirement's marker once the root sets `wiring`: on the line of the component that owns the marker's file. That change breaks two tests in `orchestrator.run`'s files, which the build of `check.coverage` may not edit (`path-ownership`):

- `crates/clm-run/tests/run.rs::an_obsolete_test_in_another_components_file_is_listed_apart`, and probably its namesake in `crates/clm-run/tests/prompt.rs`. They expect `b`'s prompt to list `b/gone` with `crates/c/t.rs:3` "in `c`'s file". `pending` now puts that marker on `c`'s line, so `b`'s prompt has no `b/gone` line at all.
- `crates/clm-run/tests/run.rs::a_stale_marker_in_a_wiring_file_is_re_cited`. Its last assertion expects `` `crates/c/gone.rs:1` in `c`'s file `` in `b`'s prompt, for the same reason.

The deferred item `backlog/clm-run-lists-an-obsolete-requirement-s-foreign-markers-no-rule-states.md` sequences it the other way round: `pending-report` goes current first, then `orchestrator.run` removes its foreign-marker code and those tests. But `check.coverage`'s land runs `clm-run`'s tests, so that order can't land. The tests can't go first either, because `clm land` refuses to let a build lose a citing test unless an approved `deleted-tests.txt` names it (`land/deleted-tests-approved`).

What the spec branch needs to do: a change whose `deleted-tests.txt` names the two `an_obsolete_test_in_another_components_file_is_listed_apart` tests, plus a requirement of `orchestrator.run` that drops `a_stale_marker_in_a_wiring_file_is_re_cited`'s check of the obsolete marker in `crates/c/gone.rs`. That moves `orchestrator.run` off the old placement first, so `orchestrator.run` builds before `check.coverage` and this item is then resolved. Keeping the foreign-marker code until `check.coverage` lands is harmless.

The spec-auditor also found a caller gap the same spec branch should settle. Once an obsolete line is split, `pending` puts the requirement's own ID in its own component's line's `waits_on`. Two callers read that ID by its prefix or as a map key:
- `effective_priorities` in `crates/clm-orchestrator/src/queue.rs` seeds the own component instead of the components that hold the other lines (`priority-orders-nodes`);
- `crates/clm-view/src/blockers.rs` keys by ID, which folds an ID's several lines into one.
