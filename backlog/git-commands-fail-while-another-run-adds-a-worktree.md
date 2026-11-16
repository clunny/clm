---
title: "Git commands fail while another run adds a worktree"
kind: bug
components: [vcs]
---

While `git worktree add` is writing a new entry under `.git/worktrees/<name>/` (its `gitdir` written, its `commondir` still empty), every git command that walks all worktrees fails on it with `fatal: failed to read .git/worktrees/<name>/commondir: Success`: `git worktree list` (git 2.43) and `git fetch`'s connectivity check among them. Builds, drafts, lands and verify runs add and remove worktrees of one clone all the time, so any of them can fail another's command. `clm-git`'s `fetch` already survives it by running again; `clm-git`'s `worktrees` doesn't, and `clm-land`'s `runner::places_of` (every land's check), `clm-verify`'s `tree::checkout`, `clm-run`, `clm-triage`, `clm-checkpoint`, `clm-orchestrator` and others call it. `clm-verify`'s `tree::places_of` stopped listing worktrees for the `verify-tree` race (`backlog/a-verify-test-fails-when-two-runs-list-worktrees-at-once`, fixed on `impl/verify`). Expected: listing worktrees holds against a worktree being added at the same moment, for instance by running again as `fetch` does, and a caller that only needs the main worktree finds it from the common git directory.
