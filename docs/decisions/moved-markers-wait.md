---
title: "An obsolete marker of a moved requirement waits for the build that re-cites the move"
date: 2026-10-31
status: accepted
---

When a split moves a requirement (removed from `a`, added to `a.b`), the tests citing `a/x` go `obsolete` on `a` and `a.b/x` is `specd`. Both nodes were then pending and ready at once, since `depends_on` orders nothing (`needs-alone-order`). But the only right work on those markers is the move: the build that brings `a.b/x` up re-points them, moving the tests with it. A build of `a` could only delete them, breaking the move, or do nothing. Twice in one session the queue offered such a parent: once beside its ready child (the parent's build stopped and reset its branch, and the child's landing left the parent with nothing pending), and once while the child itself was blocked by a backlog item (`plugin.hooks` before `plugin.hooks.guard`), so the parent could make no progress at all.

So an obsolete requirement is blocked by each pending requirement that a move takes it to, the moves being those `check.coverage/status-report` already records for change progress (`progress-follows-moves`). It is an implicit `Needs`, derived from the change's deltas: it lands in `needs_pending`, so the queue, `clm pending`, the blockers view and effective priority treat it as they treat a written need, and a blocked target keeps the chain going to its own blocker. Once the moved requirement is current the block lifts, and a marker the move left behind is ordinary obsolete work on its node.

This replaces `nested-splits`' rule that the side which builds first follows `depends_on`: for the moved markers, the node a requirement moved to always builds first.

Known edge cases:
- A marker left behind after the move's target is current is removed by the old node's build as any obsolete marker is, though the test may still prove the moved rule; the target's build re-points every marker it finds, so this needs a missed marker.
- A requirement split to several nodes waits for all of them.

Rejected:
- **Counting the moved markers on the node the move took them to.** The work is in the right place, but an `obsolete` line would then mean "remove" on one node and "re-point" on another, which needs a new pending state or line shape, a new instruction in the build prompt, and a change to how the views count obsolete markers. The block reuses the needs machinery and changes none of them.
- **A written `#### Needs` on the parent.** A removed requirement has no block to hold one, and the spec author would have to write it on every split.
- **Leaving it to each split's Design** (`plugin.hooks` told its build to leave the markers alone). The queue still offered the node, so a build was launched only to stop.

(Amended 2026-11-12, `20261112-components-own-their-tests`: the child's build no longer re-points the parent's markers or moves its tests. It writes its own tests in its own files; the parent's obsolete lines still wait for it, and then a requirement of the parent's deletes its markers, tests and code (`coupled-changes-sequence`).)
