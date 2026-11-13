---
title: "Reconsider plugin/builder-check"
kind: question
components: [plugin]
---

`plugin/builder-check` scores 6 against the reconsider threshold of 3 (workarounds 1, rewritten 4, scenarios 1, exceptions 0).

Workaround flags:
- backlog/workaround-plugin-builder-check-1.md: Skill text the requirement states lies in another component's file

Archived changes that modified it:
- 20261104-detectors-skip-in-the-root
- 20261104-rename-adds-then-removes
- 20261106-spec-change-widens-an
- 20261112-build-loop-in-clm

Scenarios: 10 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
