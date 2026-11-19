---
title: "An obsolete requirement's wait raises its own component's priority, not the components that release it"
kind: bug
components: [orchestrator]
---

`effective_priorities` in `crates/clm-orchestrator/src/queue.rs` keys the pending requirements by ID and seeds a wait's priority on the node its ID names. An obsolete requirement pending on several components (`check.coverage/pending-report`) has one line per component under one ID, so the lines overwrite each other, and the line on its own component, which waits on its own ID, raises that component's priority instead of the components holding its other lines.

Example: a root with `wiring: []`, `b/gone` removed by a `P1` change and cited in files of `a`, `b` and `c`. `b` is blocked on its own ID; `a` and `c` hold the lines that release it, yet get no raise, so `clm next` orders them below other `P5` work.

`view/blockers-view` now follows each wait to the line `clm_coverage::waits` gives (`check.coverage/wait-lines`, `a-wait-names-a-line`); the priority walk should do the same, seeding each resolved line's `node`. Whether `orchestrator/priority-orders-components` as written already states this, or a spec change must, is for the fix to judge (`code-fix-needs-a-written-rule`). Found by the code review of the `view` build that made the blockers view follow lines.
