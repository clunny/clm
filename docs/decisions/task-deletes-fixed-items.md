---
title: "The task that fixes a resolved backlog item deletes it"
date: 2026-10-22
status: accepted
---

A change's `resolves` lists the backlog items it fixes. Until now only its `finish` could delete them, because the model rejected a live change whose `resolves` named a missing item. A task in another change blocked on such an item (`blocked-by-backlog`) then waited for the whole change to finish, even when one early task had already fixed it. On 2026-10-22 that deadlocked `20261022-web-ui` and `20261022-web-toolchain`.

Now the task whose work fixes an item deletes it in its own PR, which `gate/backlog-additions` already rated `code`. The `plan` session's notes name that task for each item, or `finish` when no single task fixes it, and `finish` deletes what is left. The model accepts a live change whose `resolves` names a deleted item. To keep catching typos, the gate tests each entry a branch adds to `resolves` against the backlog at the merge base and at `HEAD` (`gate/resolves-named-items`). The model can't do that check, because it sees one tree and no history.

Alternatives considered:
- Recording which task fixes each item, as a field in the status file, and lifting the block once that task is `done`. That's a new field to keep correct, and the item would linger in `backlog/` after it's fixed.
- Keeping the model check only for changes not yet `applied`. Spec PRs apply in the same PR (`apply-in-spec-pr`), so the check would never see an unapplied change at merge time.
