---
title: "Reconsider machine/machine-report"
kind: question
components: [machine]
---

`machine/machine-report` scores 5 against the reconsider threshold of 3 (workarounds 1, rewritten 3, scenarios 1, exceptions 0).

Workaround flags:
- backlog/workaround-machine-machine-report-1.md: the auditor found a scenario the code can't observe: available, not free

Archived changes that modified it:
- 20261112-machine-cells-keep-their-state
- 20261119-wave-1-by-the-rule
- 20261120-the-plan-ends-with-phase-5

Scenarios: 13 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
