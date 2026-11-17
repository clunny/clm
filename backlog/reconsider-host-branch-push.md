---
title: "Reconsider host/branch-push"
kind: deferred
components: [host]
---

`host/branch-push` scores 3 against the reconsider threshold of 3 (workarounds 1, rewritten 2, scenarios 0, exceptions 0).

Workaround flags:
- backlog/workaround-host-branch-push-1.md: Design's git push --porcelain loses git's refusal reason through clm-git

Archived changes that modified it:
- 20261113-human-layer-rewrite
- 20261116-clm-branch-delete-refuses

Scenarios: 6 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.

## The human's answer (2026-11-17)

Keep the requirement and change its Design: `host/branch-push`'s behaviour is right and tested, and only the Design is wrong about how, naming `git push --porcelain` where the code asks `origin` with `git ls-remote` before the push (`backlog/workaround-host-branch-push-1`). The Design names the `ls-remote` check and the narrow window it leaves; `vcs` is not changed to carry a failed command's standard output for this one caller.

Due in the plan's phase 5 (`backlog/a-passing-check-proves-less-than-the-spec-claims`), when the spec-slimmer runs over `host`: that spec change rewrites the Design, deletes the workaround item and this one.
