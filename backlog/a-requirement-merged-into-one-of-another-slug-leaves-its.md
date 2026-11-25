---
title: "A requirement merged into one of another slug leaves its obsolete markers unblocked while the target is blocked"
kind: bug
components: [check.coverage]
---

`moved-markers-wait` makes an obsolete requirement wait on the pending requirement a move took it to, but `moves` (`crates/clm-coverage/src/status.rs`, `moves`) counts only a removal and an add of the same slug on another component. A change that merges several requirements into one of a new slug, in the same component, records no move.

Seen in wave 6 (`20261124-wave-6-app-by-the-rule`): `web/live-binding`, `web/replica-binding` and `web/no-cron-trigger` merged into `web/worker-deploy`, whose tests are to be re-cited there (the change says "Tests re-cited; none deleted"). `worker-deploy` is blocked (Needs `web.worker/replica-class-export`, behind), so the three obsolete lines can be neither re-cited (that would make a blocked requirement current out of order) nor deleted (no `deleted-tests.txt` line). Yet they are unblocked in `clm pending web`, so the queue keeps offering `web` as ready: three unattended `web` builds in a row have each left the same three lines pending, which is the case `moved-markers-wait` was written to stop ("The queue still offered the node, so a build was launched only to stop").

What would fix it: a way for a change to declare that removed requirements merged into a named one (or a move rule that reads it), so the obsolete lines wait on the target as a same-slug move's do. Needs a spec change to `check.coverage` (what a move is).
