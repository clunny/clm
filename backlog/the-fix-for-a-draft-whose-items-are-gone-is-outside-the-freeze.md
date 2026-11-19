---
title: "The fix for a printed draft whose items are gone is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
blocks: [orchestrator.loop/step-actions, orchestrator.loop/loop-start, orchestrator.loop/draft-of-gone-items-dropped]
choices:
  - "Build it now, as an exception"
  - "Wait for the plan's end"
---

`20261119-the-build-loop-keeps` has the build loop finish and forget a draft it printed that no subagent has started, when every item it drafts has left `origin/main` and it pushed no spec branch, instead of printing its `launch` at every step. It is not the work of `backlog/a-passing-check-proves-less-than-the-spec-claims`, whose freeze holds for everything the human doesn't except. Until it is built, such a draft costs one session that only finds its items gone, and a loop whose session holds the launch back can't drain. Once `draft-waits-for-its-build` is built, the case reported (a bug draft printed beside the build that then fixes its bug) is rarer, but a draft printed before its component's build, or one whose items another clone or the human resolves, still arises. Should `orchestrator.loop`'s next build build `step-actions`, `loop-start` and `draft-of-gone-items-dropped` now, or should they wait for the plan's end, this item then blocking them as the plan item does `draft-waits-for-its-build`?
