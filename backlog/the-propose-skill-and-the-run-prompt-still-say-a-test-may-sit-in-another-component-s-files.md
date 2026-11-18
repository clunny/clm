---
title: "The propose skill and the run prompt still say a test may sit in another component's files"
kind: deferred
components: [plugin, orchestrator.run]
origin: 20261117-phase-4-s-move-is-over
---

Since `gate/own-files` was built without the clause "a file that cites the component", no test of one component can be kept in another's files once the root names its `wiring`: `clm check` reports the marker and `clm land` refuses the edit. Two fixed texts still describe that case as open:

- `plugin/skills/propose/SKILL.md`, under Code fixes: "a fix to a component's test kept under another component's `paths` claims the component the test cites", with the reference to `fixes-count-where-cited` before it.
- The run prompt's fixed text in `crates/clm-run/src/lib.rs`: "Re-citing a marker of your own requirement, wherever its file sits, is allowed", said whether or not the root names `wiring`, against `orchestrator.run`'s Design.

This is what is left of the item that asked for phase 4's closing branch; `20261117-phase-4-s-move-is-over` did the rest. Skill and prompt text is source its component owns (`prompt-text-is-source`), so each is a code fix under its own `clm claim <component> --fix`, and the second deletes this item.
