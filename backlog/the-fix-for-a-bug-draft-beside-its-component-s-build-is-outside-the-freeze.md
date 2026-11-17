---
title: "The fix for a bug draft launched beside its component's build is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
blocks: [orchestrator.loop/plan-order, orchestrator.loop/draft-waits-for-its-build]
choices:
  - "Build it now, as an exception"
  - "Wait for the plan's end"
---

`20261117-the-build-loop-drafts` has the build loop hold back the draft of a `group` line whose components a build of the loop holds or is about to launch, with the reason `waits for build:<component>`, so the draft's `clm claim <component> --fix` is no longer refused by the loop's own build and the draft isn't relaunched into the same refusal. It is not the work of `backlog/a-passing-check-proves-less-than-the-spec-claims`, whose freeze holds for everything the human doesn't except. Until it is built, such a draft costs one session that lands nothing, and its bugs wait for a later draft. Should `orchestrator.loop`'s next build build `plan-order` and `draft-waits-for-its-build` now, or should they wait for the plan's end, this item then blocking them as the plan item does `pause`?
