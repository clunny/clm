---
title: "The draft prompt doesn't say to record a merged requirement in moves"
kind: deferred
components: [orchestrator.triage]
origin: 20261124-a-requirement-merged-into
---

Condition: `model/move-records` and `check.coverage/status-report` (change `20261124-a-requirement-merged-into`) are current, so a `change.md` setting `moves` loads and the obsolete line waits.

The draft prompt's design step (`crates/clm-triage/src/draft.rs`, the sentence that writes each test a build is to delete into `changes/<id>/deleted-tests.txt`) says nothing of a requirement merged into one of another slug. Add: when a delta removes a requirement whose sentences another requirement of a different slug now states (added or modified by the change, on the same component or another), record the pair in `change.md`'s `moves` (`requirement: <component>/<slug>`, `to: <component>/<slug>`), so the removed requirement's obsolete markers wait for the build that re-cites them (`moved-markers-wait`), and list none of its tests for deletion. Prompt text, which no requirement states (`prompt-text-is-source`): fixed by component with `clm claim orchestrator.triage --fix`, no test.
