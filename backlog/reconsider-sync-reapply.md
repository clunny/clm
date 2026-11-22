---
title: "Reconsider sync/reapply"
kind: question
components: [sync]
---

`sync/reapply` scores 5 against the reconsider threshold of 3 (workarounds 1, rewritten 3, scenarios 1, exceptions 0).

Workaround flags:
- backlog/workaround-sync-reapply-1.md: Take-back skipped on a rename branch pending a spec change

Archived changes that modified it:
- 20261106-rebase-reapplies-changes-by
- 20261109-regenerate-tests-its-own-api
- 20261121-wave-3-sync-by-the-rule

Scenarios: 16 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
