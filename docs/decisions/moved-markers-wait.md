---
title: "An obsolete marker of a moved requirement waits for the build that re-cites the move"
date: 2026-10-31
status: accepted
---

When a split moves a requirement (removed from `a`, added to `a.b`), the tests citing `a/x` go `obsolete` on `a` and `a.b/x` is `specd`. Both nodes are then pending and ready at once, since `depends_on` orders nothing (`needs-alone-order`). But the markers can only go once the move is built: the build that brings `a.b/x` up writes the moved code and its tests in its own files, and only then does a requirement of the parent's delete the parent's markers, tests and code (`coupled-changes-sequence`). A build of `a` that ran first could only delete them, leaving the rule unproven, or do nothing. Twice in one session the queue offered such a parent: once beside its ready child (the parent's build stopped and reset its branch, and the child's landing left the parent with nothing pending), and once while the child itself was blocked by a backlog item (`plugin.hooks` before `plugin.hooks.guard`), so the parent could make no progress at all.

So an obsolete requirement is blocked by each pending requirement that a move takes it to, the moves being those `check.coverage/status-report` already records for change progress (`progress-follows-moves`). It is an implicit `Needs`, derived from the change's deltas: it lands in `needs_pending`, so the queue, `clm pending`, the blockers view and effective priority treat it as they treat a written need, and a blocked target keeps the chain going to its own blocker. Once the moved requirement is current the block lifts, and the markers the move left behind are ordinary obsolete work on the old node, whose build deletes them with their tests and its copy of the code.

So for a moved requirement's markers the node it moved to always builds first, whatever `depends_on` says (`nested-splits`).

Known edge case: a requirement split to several nodes waits for all of them.

Rejected:
- **Counting the moved markers on the node the move took them to.** An `obsolete` line would then mean one kind of work on one node and another on the next, which needs a new pending state or line shape, a new instruction in the build prompt, and a change to how the views count obsolete markers; and the markers sit in the old node's files, which the other node's build doesn't change (`path-ownership`). The block reuses the needs machinery and changes none of them.
- **A written `#### Needs` on the parent's removed requirement.** A removed requirement has no block to hold one, and the spec author would have to write it on every split.
- **Leaving it to each split's Design** (`plugin.hooks` told its build to leave the markers alone). The queue still offered the node, so a build was launched only to stop.
