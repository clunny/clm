---
status: applied
resolves: []
---

## Intent

On the backlog, the change sets the `components` of two bugs filed on `orchestrator.loop` to the components their fixes lie in, and says in each item what was checked. It has no delta: both fixes are code-only, since a requirement as written already states the behaviour each restores, and neither lies in the build loop, so `clm claim --fix` can't take them from here (`code-fix-resolves-bug`). The next `clm triage` groups them by their corrected components.

- `backlog/clm-build-returned-fails-when-its-wrap-up-deletes-a-branch-a` goes to `orchestrator.checkpoint`. `orchestrator.checkpoint/wrap-up-delete-loses-race` already says a branch `origin` no longer has counts as done when `origin` refuses its delete. `clm wrap-up` reads only the refusal git makes of a stale lease, and not the remote's own (`[remote rejected] ... cannot lock ref`), which it gets when the branch goes between the push's read of `origin` and its update. The loop is right to exit 2 when `clm wrap-up` does. The item's second question is answered in it: deleting that `propose/` claim is intended, since the draft's fix had landed and deleted the item, and `orchestrator.checkpoint/checkpoint` deletes a claim whose item `origin/main` no longer holds.
- `backlog/the-build-loop-prints-a-run-action-again-while-the-same-command` goes to `plugin`. The loop already does what the item expects: `orchestrator.loop/step-actions` prints a run still `planned` again until `clm build launched` reports it, and `clm build launched <name> <task-id>` records a `run` action's run (`verify`, `land:<batch>`) as running like any other. The `build` skill's `run` bullet doesn't tell the session to report it, as its `launch` bullet does. Recording a `run` as running when it is printed was not taken: an action line lost to compaction would never be printed again, which is what the acknowledgement guards against.

## Edge cases and workarounds

None.
