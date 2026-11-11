---
title: "Builds install web dependencies by hand because no build setup step is declared"
kind: bug
components: []
---

During /clm:build, unattended builds of components with JavaScript suites (plugin.panel, web.client-live) found their fresh worktrees had no installed node dependencies. The repository's clm.toml declares no `[build] setup`, so `clm setup` does nothing and each build ran `npm ci` by hand in the affected directory before its tests could run. The build prompt's `tests-run-in-their-own-tree` rule expects the declared setup step to prepare a worktree; here each build has to notice the gap and work around it itself.

Expected: a build's worktree is prepared by the declared setup step (or the prompt tells a build how to prepare a directory the setup doesn't cover), so no build improvises dependency installs.
