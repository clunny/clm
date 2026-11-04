---
title: "Sessions watch their PRs with clm watch and carry on, instead of waiting for the human"
date: 2026-10-22
status: superseded
superseded_by: land-locally
---

`clm watch <pr>` polls a PR and exits only when it needs attention: `merged`, `closed`, `conflict`, or `failed` with the checks. A session runs it as a background command, so Claude Code wakes it when it exits. No polling loop in the conversation and no scratch scripts, which is what replaced the human reporting merges by hand.

A `HARD` PR fails the `gate` check with `gate/approval` until a human approves it, and GitHub's check state doesn't say why the gate failed. So on a failed `gate`, `clm watch` reads the violations from that gate job's log, and keeps waiting when `gate/approval` is the only one. It reads CI's verdict rather than running the gate locally, because CI's gate is built from the PR's own source.

When an item's PR fails or conflicts after its session ended, `implement` re-runs `clm run` for it once, which resumes in its worktree. It doesn't try to work out the cause: a stale basis usually shows as a failed `merge` check, and the resumed run's `clm sync` finds it.

`propose` watches its spec PR through to merge and handles conflicts and failures on the way. Once it merges, it doesn't start building: it ends with a summary and suggests a new session for `/clm:implement`, so the build starts from a clean context holding only the merged specs.
