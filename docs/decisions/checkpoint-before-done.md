---
title: "A session says it is done only after clm checkpoint passes"
date: 2026-10-22
status: accepted
---

Before a session tells the human that everything is saved and the session can be ended or cleared, it runs `clm checkpoint`. That command:
- removes merged worktrees and branches, locally and on `origin`, and the `autosync/spec/<id>` snapshots of changes that are in `main`;
- reports any work CLM's flow made that isn't safe on `origin` (`dirty`, `unpushed`);
- runs `clm check` on `origin/main`.

A branch counts as merged when its current tip is in `origin/main` but not on `origin/main`'s first-parent chain: its commits reached `main` through a merge, `clm land`'s or a GitHub pull request's from before the cutover. "Its tip is in `main`" alone would also match a branch just created from `main`, and deleting that could lose someone's new work; such a branch's tip is a commit `main` itself was at, on the first-parent chain, so it is kept.

It looks only at work the flow makes: the builds' worktrees, `.claude/worktrees/agent-*` directories on `impl/*` branches, which is where Claude Code puts a subagent's worktree (`builds-in-subagents`), and `spec/*` and `impl/*` branches. The human's own files, branches and worktrees are left alone: a worktree whose directory isn't named `agent-*` is never removed, and a removed worktree is always one on a merged branch with nothing uncommitted.

A session fixes what is its own and doesn't loop on what isn't. The checkpoint draws that line itself: only the checkout it runs in and the branch checked out there can make it incomplete. Unsafe work elsewhere, a build's worktree with uncommitted changes or another session's spec branch in the main checkout, is listed as `elsewhere` for the human, like `open`. Without that, any session running in parallel with another could never pass. When the checkpoint can't pass, it ends with the summary and the reasons, without the "safe to clear" line.

The test for an autosync snapshot is whether its change is in `main`, not whether its spec branch exists. Autosync pushes before a spec branch's first push, and a branch may already have been deleted elsewhere. A snapshot of a spec PR that closed unmerged is kept, since it may be the only copy of that work.

Updated 2026-10-25: builds used to run in a `.clm/worktrees/` shelf that `clm run` made, and the session's own work was whatever sat under its shelf. `20261024-node-builds-as-subagents` moved builds into Claude Code's worktrees, and this text now describes that (`orchestrator.checkpoint`).

Updated 2026-10-26: a branch counted as merged first when GitHub reported a merged PR at its tip, then, after the cutover to `clm land`, when a `landed` record named it at its tip. The record test left every branch GitHub merged before the cutover reported as unpushed and never removed. `20261026-checkpoint-merged-without-record` replaced both with the ancestry test above, which git answers offline for every merge, whoever made it. A squash-merged branch would fail it and be kept, which loses nothing; neither the pull requests nor `clm land` ever squash-merged.
