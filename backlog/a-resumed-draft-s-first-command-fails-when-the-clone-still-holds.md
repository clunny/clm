---
title: "A resumed draft's first command fails when the clone still holds the draft's local branch"
kind: bug
components: [orchestrator.triage]
origin: 20261113-no-test-reads-what
---

The prompt `clm draft --resume` writes tells the session to run `git switch -c spec/<id> --no-track origin/spec/<id>`. In the clone that ran the first draft session, the local branch `spec/<id>` outlives that session's removed worktree, so the command fails with `fatal: a branch named 'spec/<id>' already exists`, and the prompt says nothing about that case. Met resuming `20261113-no-test-reads-what`: the local branch was at the same commit as `origin/spec/<id>` and checked out nowhere, so `git switch spec/<id>` carried on, but the session had to work that out for itself.

The prompt's command should work whether or not the local branch exists, and leave the session on the commit `origin` holds (for example `git switch -C spec/<id> --no-track origin/spec/<id>`, or `clm draft --resume` deleting a stale local branch that no worktree holds before it writes the prompt). The prompt's text is at `crates/clm-triage/src/draft.rs`, and `crates/clm/tests/draft.rs` asserts the command as written.
