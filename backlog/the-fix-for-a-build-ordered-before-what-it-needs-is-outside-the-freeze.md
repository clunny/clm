---
title: "The fix for a build ordered before what it Needs is outside the plan's freeze"
kind: question
components: [orchestrator, orchestrator.loop]
blocks: [orchestrator/next, orchestrator.loop/release]
choices:
  - "Build it now, as an exception"
  - "Wait for the plan's end"
---

`20261119-the-build-loop-launches` has `clm next` list, at equal priority, every component with no pending requirement that waits on other work before any that has one, so the build loop, which takes `clm next`'s order and cap, builds a component such as `model` before one such as `apply` whose requirements Need it, and `apply` is built once instead of twice. It also lets `clm build release` give up a build the loop printed and the session never launched. It is not the work of `backlog/a-passing-check-proves-less-than-the-spec-claims`, whose freeze holds for everything the human doesn't except. Until it is built, each wave of phase 5 whose components Need one another can build a component for its unblocked requirements and again once what it Needs lands, one extra build each time, and a session that wants to give up a printed build has to report a return for a run that never started. Should `orchestrator`'s and `orchestrator.loop`'s next builds build `orchestrator/next` and `orchestrator.loop/release` now, or should they wait for the plan's end, this item then blocking them as the plan item does `orchestrator.loop/plan-order`?
