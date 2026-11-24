---
title: "The push guard lets a push through whose settings come by GIT_CONFIG_COUNT or GIT_CONFIG_PARAMETERS"
kind: bug
components: [plugin.hooks.guard]
---

`push-guard` reads a push's `-c` and (with `20261123-the-push-guard-lets`) `--config-env` settings under `push.`, `remote.` and `branch.`, and `push-guard-scoped-to-clm` counts a `core.worktree` or `core.bare` set by either as naming another work tree. A segment's assignment of `GIT_CONFIG_COUNT` with `GIT_CONFIG_KEY_<n>`/`GIT_CONFIG_VALUE_<n>`, or of `GIT_CONFIG_PARAMETERS`, sets the same settings, but the guard neither passes them to its queries nor reads them, and the Design's list of what gets through doesn't name them. So `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push GIT_CONFIG_VALUE_0=HEAD:main git push` from a branch is allowed while its `-c` form is blocked. Needs a spec change: either read these assignments as the `-c` options are, or name them in the Design among what a backstop lets through. Found by the builder check of `20261123-the-push-guard-lets`.
