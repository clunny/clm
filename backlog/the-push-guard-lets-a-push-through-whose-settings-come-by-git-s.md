---
title: "The push guard lets a push through whose settings come by git's --config-env"
kind: bug
components: [plugin.hooks.guard]
---

`push-guard` passes a push's `-c` settings under `push.`, `remote.` and `branch.` to its queries, and `push-guard-scoped-to-clm` counts a `-c core.worktree` or `core.bare` as naming another work tree, but neither says anything of git's `--config-env <name>=<envvar>`, which sets the same settings from an environment variable. The requirement says only that `--config-env`'s value is skipped when finding the subcommand.

So `B=HEAD:main git --config-env remote.origin.push=B push` from a branch, or `U=upstream git --config-env push.default=U push` on a branch tracking `main`, is allowed while the `-c` forms are blocked; and `W=/clm/checkout git --config-env core.worktree=W push origin main` from a scratch repository is taken as outside CLM. The guard can't read the variable's value (it may be set by the segment or inherited), so the safe reading is probably: a `--config-env` (separate or glued `--config-env=`) whose name, without regard to ASCII case, starts with `push.`, `remote.` or `branch.`, or is `core.worktree` or `core.bare`, makes the push count as under CLM and as reaching `main` when it names no other branch, or at least the latter two make it under CLM. Needs a spec change to `push-guard` and `push-guard-scoped-to-clm`. Found by code review on impl/plugin.hooks.guard; not a regression (before this build the guard read the setting as git's subcommand and judged no push at all).
