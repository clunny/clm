---
title: "A session's raw git push of a branch is refused by nothing"
kind: idea
components: [plugin.hooks.guard]
---

Sessions push with `clm branch push`, which overwrites only what the clone last saw, and delete remote branches with `clm branch delete`. The guard blocks only a `git push` that reaches `main` (`plugin.hooks.guard/push-guard`); a raw `git push` or `git push --force` of any other branch is allowed.

Until `20261113-phase-2-delete`, four requirements pinned the sentence that tells a session to use `clm branch push`: in the skills, in the prompts `clm run` and `clm draft` write, and in this repository's `AGENTS.md`. They were scans of prompt text and are deleted. If the instruction must hold, the guard is the program that can hold it: refuse a session's `git push` under CLM and name `clm branch push`.
