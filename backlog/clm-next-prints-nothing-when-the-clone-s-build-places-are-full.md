---
title: "clm next prints nothing when the clone's build places are full, so /clm:build reads a full cap as an empty queue"
kind: bug
components: [orchestrator, plugin.build]
---

## What happened

`/clm:build` ran `clm next`. It printed nothing and exited 0. Going by its steps, the skill took that as an empty queue and went on to the wrap-up. But `clm pending --at origin/main` still listed pending, unblocked requirements, and `clm view blockers` showed `config` as ready. The real cause: another session's two triage drafts held both build places (`max_builds = 2`). Finding that out took manual digging through `git worktree list` and `clm config list`.

## Expected

The build skill tells the human "the cap is reached and by how many places, naming `max_builds`" only when `clm run` exits 3. It never runs `clm run` when `clm next` prints nothing. So that message can never appear when the cap is already full before the first build. `clm next` (`build-places`: "nothing when it has none") should say on stderr that places are full, for example `no free build place: 2 of max_builds = 2 in use`, or exit with its own code. The build skill should then report the full cap instead of an empty queue.

## Rules involved

`build-places`, `plugin/build` cap handling, `next-wait` (a `--wait` watch would also sit silently while the cap is full).
