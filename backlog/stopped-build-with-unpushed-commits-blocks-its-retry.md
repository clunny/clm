---
title: "Stopped build with unpushed commits blocks its retry"
kind: bug
components: [orchestrator.run, plugin]
---

Step: /clm:build, retrying a build that stopped without landing (step 3).

What happened: a build subagent committed its work on impl/<component> after its `clm rebase` had rewritten the claim commit onto a newer main, then stopped before `clm branch push` (its `clm land --check-only` was killed for low memory). `clm run --finish <component>` freed the place and, since the branch had not landed, kept the build's worktree, which Claude Code had locked and which still had impl/<component> checked out. The retry's `clm run <component> --review-model <alias>` then exited 1: the local branch isn't landed and doesn't contain origin/impl/<component>, "reconcile or delete it". The build skill's retry rule says only to run `clm run` again with a new subagent; it names no way to reconcile, and exit 1 tells the session to launch nothing. Recovering took pushing the stopped build's branch by hand from its leftover worktree with `clm branch push`, then unlocking and removing that worktree so a new build could switch to the branch.

Expected: a stopped build's committed but unpushed work survives into its retry without hand work. For example `clm run --finish` pushes a stopped build's branch (as its sync point would) or names the step, or `clm run` accepts a local branch whose only divergence from origin is a rebased claim, and the build skill says what to do when `clm run` refuses a retry this way.

Rules involved: builds-in-subagents, resume on the component's branch (versioned-markers), the retry rule in the build skill's step 3.
