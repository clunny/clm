---
title: "A build's tests can run against the main checkout's dependencies instead of its own worktree's"
kind: bug
components: [orchestrator.run, plugin.build, config]
issues: [406]
---

Each build runs in a worktree under `.claude/worktrees/`, inside the main checkout. A fresh worktree has nothing installed. Toolchains whose lookup reaches the parent folder resolve the main checkout's packages instead: Node walks up to its `node_modules`, and a shared Python virtualenv with an editable install imports the main checkout's source. The build's tests then run against code that isn't the branch's, so a test-first red or green phase can mean nothing. Issue #406 saw this three times in one session with Node.

Expected: a general rule that a build's tests run against its own worktree's code and dependencies. Before a build's first test run, CLM runs a setup step the project declares (as `[land] check` is declared), and never names a toolchain. The project's command holds the details for its toolchain. Whether `clm init` seeds a default for each language, and what existing projects are offered, is part of the design.
