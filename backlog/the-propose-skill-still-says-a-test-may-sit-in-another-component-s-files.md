---
title: "The propose skill still says a test may sit in another component's files"
kind: deferred
components: [plugin]
origin: 20261117-phase-4-s-move-is-over
---

`plugin/skills/propose/SKILL.md`, under Code fixes, says "a fix to a component's test kept under another component's `paths` claims the component the test cites". Since `gate/own-files` was built without the clause "a file that cites the component", no such test can be kept: `clm check` reports the marker once the root names its `wiring`, and `clm land` refuses the edit. The sentence and the reference to `fixes-count-where-cited` before it describe a case that is over.

This is what is left of the item that asked for phase 4's closing branch; `20261117-phase-4-s-move-is-over` did the rest. Skill text is source that `plugin` owns (`prompt-text-is-source`), so the fix is a code branch under `clm claim plugin --fix`, which deletes this item.
