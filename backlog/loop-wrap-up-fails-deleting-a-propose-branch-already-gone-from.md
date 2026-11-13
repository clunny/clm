---
title: "Loop wrap-up fails deleting a propose branch already gone from origin"
kind: bug
components: [orchestrator.triage]
---

Reporting a draft run's `landed <sha>` with `clm build returned` exited 2: the wrap-up it runs tried to delete the draft's `propose/<slug>` branch on origin, leased on the tip it last saw, and git rejected the delete as stale info. The branch was already gone from origin (`git ls-remote` shows nothing), so there was nothing left to delete.

Expected: deleting a `propose/<slug>` branch that no longer exists on origin counts as done, so the return is recorded without an error. The loop's state afterwards looked consistent (the run left the running list), but the command reported failure, which an adapter could read as the return not being recorded.
