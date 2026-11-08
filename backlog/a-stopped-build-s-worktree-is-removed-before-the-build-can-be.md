---
title: "A stopped build's worktree is removed before the build can be resumed"
kind: bug
components: [orchestrator.checkpoint, orchestrator.run]
---

Step: /clm:build, after builds stopped without landing (here because the disk filled during `clm land`), then `clm wrap-up` and a later `clm run <component>` to retry.

What happened: `clm run --finish <component>` kept each stopped build's worktree, as expected for an unlanded branch. Then `clm wrap-up` removed the worktree of a stopped build whose branch was pushed and clean, because it treats pushed work as safe. A later `clm run <component>` to retry another stopped component most likely removed that build's worktree too: it was listed by `git worktree list` before that command and gone after. Without its worktree, the stopped build's subagent can't be resumed with SendMessage: Claude Code refuses ("its worktree no longer exists"). So the build had to start fresh, losing its context and recompiling the workspace from nothing. Only the one stopped build whose worktree survived (its commit was unpushed, so wrap-up kept it) could be resumed.

Expected: a stopped build's worktree is the state its subagent needs to be resumed, not only a place that holds unpushed commits. `clm wrap-up` and `clm run` should keep a stopped, unlanded build's worktree while the orchestrating session may still resume that build: for example until the build lands, is retried fresh, or the session ends. Or they should at least say before removing it that doing so rules out resuming. This goes with `backlog/a-stopped-build-is-always-restarted-from-scratch-even-when`, which asks for the orchestrator to judge whether to resume. That judgement needs the worktree still there.

Rules involved: `checkpoint-before-done` (wrap-up's removals), `resume-limited-builds`, and /clm:build step 3's "it keeps its claim and what it pushed".
