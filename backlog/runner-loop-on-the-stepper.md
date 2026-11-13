---
title: "clm runner keeps a loop of its own beside the build loop's stepper"
kind: idea
components: [orchestrator.runner]
---

`clm runner` drains the queue with its own loop in Rust: `clm next`, `clm run`, a headless `claude -p` session per component, `clm run --finish`. The `20261112-build-loop-in-clm` change put the attended loop's rules in `orchestrator.loop` (`clm build`): priority across drafts and builds, resume and rebuild by cause, escalation, folds, archives, verify runs and the human's queue. The runner has none of these, so an unattended machine builds with weaker rules than an attended session, and the two loops can drift.

Move the runner onto the stepper: the runner carries out `clm build`'s actions with `claude -p` sessions as the build skill does with subagents, once `orchestrator.loop` is built. It needs a spec change to `orchestrator.runner`'s requirements; left out so the build-loop change stays one reviewable piece.
