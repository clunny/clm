---
title: "A session says it is done only after clm wrap-up passes"
date: 2026-10-22
status: accepted
---

Before a session tells the human that everything is saved and the session can be ended or cleared, it runs the checkpoint, `clm wrap-up`. That command:
- removes landed worktrees and branches, locally and on `origin`, and the `autosync/spec/<id>` snapshots of changes that are in `main`;
- reports any work CLM's flow made that isn't safe on `origin` (`dirty`, `unpushed`);
- runs `clm check` on `origin/main`.

A branch counts as landed when its current tip is in `origin/main` but not on `origin/main`'s first-parent chain: its commits reached `main` through a merge, `clm land`'s or a GitHub pull request's, whether or not a `landed` record names it. Git answers that offline for every merge, whoever made it. "Its tip is in `main`" alone would also match a branch just created from `main`, and deleting that could lose someone's new work; such a branch's tip is a commit `main` itself was at, on the first-parent chain, so it is kept. A squash-merged branch would fail the test and be kept, which loses nothing; neither the pull requests nor `clm land` ever squash-merged.

It looks only at work the flow makes: the builds' worktrees, `.claude/worktrees/agent-*` directories on `impl/*` branches, which is where Claude Code puts a subagent's worktree (`builds-in-subagents`), and `spec/*` and `impl/*` branches. The human's own files, branches and worktrees are left alone: a worktree whose directory isn't named `agent-*` is never removed, and a removed worktree is always one on a landed branch with nothing uncommitted.

A session fixes what is its own and doesn't loop on what isn't. The checkpoint draws that line itself: only the checkout it runs in and the branch checked out there can make it incomplete. Unsafe work elsewhere, a build's worktree with uncommitted changes or another session's spec branch in the main checkout, is listed as `elsewhere` for the human, like `open`. Without that, any session running in parallel with another could never pass. When the checkpoint can't pass, it ends with the summary and the reasons, and `wrap-up incomplete` in place of `wrap-up ok`.

The test for an autosync snapshot is whether its change is in `main`, not whether its spec branch exists. Autosync pushes before a spec branch's first push, and a branch may already have been deleted elsewhere. A snapshot of a spec PR that closed unmerged is kept, since it may be the only copy of that work.

Rejected, for the test of a landed branch: asking GitHub whether a merged pull request sits at the tip, which needs the host and misses `clm land`'s merges; and requiring a `landed` record at the tip, which left every branch merged before `clm land` existed reported as unpushed and never removed.
