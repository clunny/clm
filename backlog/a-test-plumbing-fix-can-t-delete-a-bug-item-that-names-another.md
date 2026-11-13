---
title: "A test-plumbing fix can't delete a bug item that names another component"
kind: bug
components: [orchestrator.triage]
origin: 20261113-flaky-panel-test-fails
---

The draft prompt's test-plumbing route says to fix the plumbing on a branch of one's own, delete the item in a commit whose trailer holds `Resolves: backlog/<slug>`, and land once `clm gate --base origin/main` prints `code`. `human-layer-proof` says the gate accepts that trailer on any branch.

It doesn't when the item names the wrong component. `backlog/flaky-panel-test-fails-a-land-removing-its-temp-directory` named `plugin.panel`, but the failing test and its fake engine are under `capabilities/panel/`, which is `capability`'s. The fix branch changed only those files and deleted the item with the trailer, and the gate printed `mixed` (`gate/classify`), since the deletion counts only for an item on a component the branch's code touches.

The prompt gives a components correction only for a code-only bug fixed by `clm claim --fix`, and says nothing for test plumbing. The draft corrected the item's `components` to `capability` on its spec branch, landed that, and then landed the fix; the prompt should say what a draft does here, or the gate should accept the deletion for test plumbing whatever component the item names.
