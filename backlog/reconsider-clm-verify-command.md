---
title: "Reconsider clm/verify-command"
kind: question
components: [clm]
---

`clm/verify-command` scores 3 against the reconsider threshold of 3 (workarounds 1, rewritten 2, scenarios 0, exceptions 0).

Workaround flags:
- backlog/workaround-clm-verify-command-1.md: Stop window approximated: clm-verify does not expose when its mutation pass runs

Archived changes that modified it:
- 20261109-the-after-land-verify
- 20261116-a-mutation-pass-that-stops

Scenarios: 2 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
