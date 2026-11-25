---
title: "The propose skill doesn't say to record a merged requirement in moves"
kind: deferred
components: [plugin]
origin: 20261124-a-requirement-merged-into
---

Condition: `model/move-records` and `check.coverage/status-report` (change `20261124-a-requirement-merged-into`) are current, so a `change.md` setting `moves` loads and the obsolete line waits.

`plugin/skills/propose/SKILL.md`, the bullet on `deleted-tests.txt` (around line 31), says to leave out the tests of a requirement that was merged or reworded, since its build re-cites them. Add beside it: when a delta removes a requirement whose sentences another requirement of a different slug now states (added or modified by the change, on the same component or another), record the pair in `change.md`'s `moves` (`requirement: <component>/<slug>`, `to: <component>/<slug>`), so the removed requirement's obsolete markers wait for the build of the requirement that re-cites them (`moved-markers-wait`). A same-slug move to another component needs no entry. Prompt text, which no requirement states (`prompt-text-is-source`): fixed by component with `clm claim plugin --fix`, no test.

At the same time, on a branch of its own that changes only `docs/` (`small-change-no-claim`), add `moves` to `docs/spec-format.md`'s list of `change.md` keys and to its `moved-markers-wait` sentence (a requirement merged into one of another slug and recorded in `moves` waits too): the guide `clm guide spec-format` is built from that file, so it waits for the key to load (`checks-relax-before-use`).
