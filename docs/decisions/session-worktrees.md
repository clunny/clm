---
title: "A skill cuts each branch of the human's session in a worktree of its own"
date: 2026-10-26
status: accepted
---

`propose`, `init` and `implement`'s archive step cut their branches with `git worktree add .claude/worktrees/<name> -b <branch> origin/main` and work there, instead of switching the checkout the session started in. When the branch has landed, the skill changes back and removes the worktree before `clm checkpoint`.

**Why.** `propose-claims` expects several proposing sessions on one clone at once, and they all start in the same main checkout. Twice a session's `git switch` to its new spec branch moved another session's `HEAD` between its commands: the other session's next commits landed on the first session's branch, autosync pushed them to its `autosync/spec/<id>`, and nothing warned either session until `clm gate` listed the other change's file. Builds and drafts never hit this, since the Agent tool's `isolation: "worktree"` gives each one its own worktree (`builds-in-subagents`). Leaving the main checkout on `main` also keeps anything that runs there after a merge, such as a local rebuild of `clm`, working.

`.claude/worktrees/` is where Claude Code already keeps subagents' worktrees, and it lies inside the session's project, so a `cd` there persists between Bash calls and the `Stop` hook, which finds the repository from the event's `cwd`, autosyncs the worktree's branch. `clm checkpoint` removes only build worktrees (`agent-*`), so the skill removes its own.

**Amended 2026-10-28** (`20261028-propose-s-worktree-step`). A session Claude Code already isolates in a worktree (its checkout lies under `.claude/worktrees/`, where the Agent tool's `isolation: "worktree"` puts a subagent) can't follow this: the isolation refuses `git worktree add` and every write outside the session's own worktree. It needs no second worktree either: the rule exists because the main checkout is shared, and a worktree under `.claude/worktrees/` is one session's own. So `propose` there cuts `spec/<id>` in the checkout it has, with `git switch -c spec/<id> origin/main`, exactly as `clm draft`'s prompt has drafts do, and at the end switches it back off the branch with `git switch --detach origin/main` rather than removing anything: the worktree belongs to whoever made it.

Alternatives considered:
- `clm claim backlog/<slug>` refusing, or warning, when the checkout's `HEAD` is on a branch another live session uses. Git doesn't record which session uses a checkout, and a warning leaves the collision possible; separate worktrees make it impossible (`deterministic-enforcement`).
- Claude Code's own worktree tool. It names the branch itself and isn't available in every harness; `git worktree add` works anywhere and names the branch CLM needs.
