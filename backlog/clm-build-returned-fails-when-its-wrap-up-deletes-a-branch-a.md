---
title: "clm build returned fails when its wrap-up deletes a branch a draft already deleted"
kind: bug
components: [orchestrator.loop]
---

Step: the /clm:build skill reporting a run's return with `clm build returned <name> --result "landed <sha>"`.

What happened: while another draft run was finishing (it had landed its change as code-only and deleted its own `spec/<id>` branch on origin), `clm build returned` for a different run ran `clm wrap-up`, which pushed deletions of that draft's `propose/<slug>` and `spec/<id>` branches. The remote rejected the `spec/<id>` deletion ("cannot lock ref ... unable to resolve reference") because the branch was already gone, and `clm build returned` exited 2 with that git error, without printing any actions. Running the same `clm build returned` again a moment later succeeded and printed the next actions.

Expected: deleting a remote branch that is already gone counts as done, so a wrap-up that races a run's own cleanup doesn't fail the return report. Also, the wrap-up deleted a live draft's `propose/` claim before that draft had returned; whether that's intended is worth checking.
