---
title: "clm build returned fails when its wrap-up deletes a branch a draft already deleted"
kind: bug
components: [orchestrator.checkpoint]
---

Step: the /clm:build skill reporting a run's return with `clm build returned <name> --result "landed <sha>"`.

What happened: while another draft run was finishing (it had landed its change as code-only and deleted its own `spec/<id>` branch on origin), `clm build returned` for a different run ran `clm wrap-up`, which pushed deletions of that draft's `propose/<slug>` and `spec/<id>` branches. The remote rejected the `spec/<id>` deletion ("cannot lock ref ... unable to resolve reference") because the branch was already gone, and `clm build returned` exited 2 with that git error, without printing any actions. Running the same `clm build returned` again a moment later succeeded and printed the next actions.

Expected: deleting a remote branch that is already gone counts as done, so a wrap-up that races a run's own cleanup doesn't fail the return report. Also, the wrap-up deleted a live draft's `propose/` claim before that draft had returned; whether that's intended is worth checking.

Checked on 2026-11-15 (`20261115-clm-build-returned-fails`): the fix is code-only and lies in `orchestrator.checkpoint`. `orchestrator.checkpoint/wrap-up-delete-loses-race` already says a branch `origin` no longer has counts as done when `origin` refuses its delete, but `delete_on_origin` in `crates/clm-checkpoint/src/lib.rs` reads only the `(stale info)` refusal git itself makes of a lease, and treats the remote's own refusal, ` ! [remote rejected] (delete) -> <branch> (cannot lock ref ...)`, as a git failure. The loop is right to exit 2 when `clm wrap-up` does. Deleting the `propose/` claim is intended: the draft's fix had landed and deleted the item, and `orchestrator.checkpoint/checkpoint` deletes a claim whose item is no longer in `origin/main`, since nothing is left for it to hold.
