---
title: "Reconsider check.coverage/test-lists"
kind: question
components: [check.coverage]
---

`check.coverage/test-lists` scores 3 against the reconsider threshold of 3 (workarounds 1, rewritten 1, scenarios 1, exceptions 0).

Workaround flags:
- backlog/workaround-check.coverage-test-lists-1.md: Stopped to raise a spec change: building it breaks land's test of the old rule

Archived changes that modified it:
- 20261118-the-pass-s-tooling

Scenarios: 13 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
