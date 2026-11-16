---
title: "Design's git push --porcelain loses git's refusal reason through clm-git"
kind: workaround
components: [host]
requirement: host/branch-push@e99935c6
---

The Design says `branch push` tells whether the push moved `origin`'s branch from `git push --porcelain`, whose line is flagged `=` when `origin` already held the commit. With `--porcelain`, git writes its per-ref status, the refusal reason included (`[rejected] (stale info)`), to stdout, and `clm_git::git`/`run` return only stderr on failure (`Error::Failed` carries no stdout). So a porcelain push would drop `stale info` from the refusal and break the scenario "moved since the last fetch". The build instead asks `origin` with `git ls-remote origin refs/heads/<branch>` before the push and records the tip only when `origin` didn't already hold it. It leaves a narrow window: another clone pushing this exact commit between the `ls-remote` and the push would be recorded as this clone's push. Either the Design should name the `ls-remote` check, or `vcs` should carry stdout in `Error::Failed` so the porcelain route works.
