---
title: "Unattended sessions correct their pull request body through clm pr body, not a gh allow rule"
date: 2026-10-22
status: accepted
---

`clm run`'s prompt has sessions write the pull request body with `Write` to `.clm/pr-body.md` and open the pull request with `clm pr open impl/<component> --title <title> --body-file .clm/pr-body.md`. To correct the body they rewrite the file and run `clm pr body .clm/pr-body.md`, `host`'s command, which sets only the body of the open pull request whose head is the current branch and does the same on every host (`host-adapters`).

**Why.** Under `dontAsk`, a multi-line inline `--body` was denied, and `gh pr edit` isn't allowed and fails on `gh` 2.45 (the Projects (classic) error), so sessions opened PRs with wrong bodies they couldn't fix. `Bash(clm *)` is already allowed, so a `clm` command adds the ability without widening what unattended sessions may run (`unattended-commands`), and it limits the call to the session's own PR and its body. A file in the worktree belongs to one item, and `Write` replaces it, so no stale body from another run carries over.

**Alternatives.** Allowing `gh api -X PATCH repos/*/pulls/*` needs no code but lets a session close, retitle or retarget any PR. Adding a guard rule to narrow it would need the guard to know the session's PR, more machinery than the command for the same result.
