---
title: "Concurrent spec lands starve each other on gate/up-to-date for derived SPEC.md files"
kind: bug
components: [land]
origin: 20261107-projects-set-up-before
---

**Step:** `clm land spec/20261107-projects-set-up-before` during a triage session, with other drafts landing spec branches at the same time.

**What happened:** the land ran its checks for minutes, then the gate blocked it with `gate/up-to-date: specs/plugin/SPEC.md changed on both this branch and origin/main; run clm rebase to regenerate it` (once also `specs/SPEC.md`), because another draft's spec branch had landed during the check and both had regenerated the same derived `SPEC.md`. `clm rebase`, `clm branch push` and `clm land` again lost the same race four times in a row. Each round costs a full check run, and nothing queues or retries.

**Expected:** a spec branch that is only behind `main` in derived files lands without a manual rebase loop. `clm land` already merges `main` in for a branch that is merely behind. It could regenerate derived `specs/` files from `changes/` as `clm rebase` does, or check the gate's up-to-date rule before the long check rather than after it, so that concurrent spec lands don't starve one another.
