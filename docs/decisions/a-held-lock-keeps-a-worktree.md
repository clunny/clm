---
title: "A worktree locked by a process that still runs is kept"
date: 2026-11-19
status: accepted
---

`clm wrap-up` removed every clean worktree in `.claude/worktrees/agent-*` whose `impl/*` branch had landed, and `clm run <component>` removed such a worktree as a landed leftover. Both took "landed" to mean "the run is over". It doesn't: a draft that fixes a bug by component lands `impl/<component>` from its own worktree and then goes on working there, and a build still writes its report after its land. Every step of the build loop runs `clm wrap-up`, so on 2026-11-19 a draft's worktree was removed under it partway through its cleanup, and it left an empty `spec/<id>` and `autosync/spec/<id>` on `origin` for the session to delete by hand (`backlog/a-running-subagent-s-clean-worktree-is-removed-under-it`).

**The rule** (`vcs/held-lock`). A worktree's lock is *held* when its reason names a process, as `pid <n>`, that still runs on this machine. `clm wrap-up` and `clm run`'s leftovers step remove no worktree whose lock is held. `clm run --finish` and `clm draft --finish` are unchanged: they run once the run has returned, and unlock first.

**Why the lock.** Git's lock is the owner's word that a worktree is in use, and Claude Code writes one on every subagent's worktree, naming its own session's process: `claude agent agent-<id> (pid <n> start <t>)`. That process is the session, not the subagent, so a worktree of a live session is kept even after its run returned; the `--finish` the loop or the `build` skill runs on each return removes those. What wrap-up is the backstop for, a run nobody finished, is a run of a session that has gone, and a session that has gone holds no process. The test is the operating system's, so it is the same in every session and on every host, and no new state is kept (`reuse-first`, `yagni`).

**What degrades.** A lock that names no process, or no lock at all, is judged as before, so an agent harness that writes no process ID loses nothing it had (`agent-harness-adapters`). A reused process ID keeps a worktree until that process ends, which loses nothing. Where CLM can't tell whether a process runs, a named process counts as running, keeping rather than removing.

Considered:
- **Have the build loop's step run `clm wrap-up` without removing worktrees.** Rejected: a wrap-up run by another session, or the human's own, removes the same worktree, since several sessions finish while others work (`checkpoint-reports-unsafe-work`).
- **Record each run's worktree in its build place, and keep a worktree whose place counts.** Rejected: a build's place stops counting when its branch lands, which is the moment in question, and a draft's place names its `spec/<id>`, not the `impl/<component>` its fix took; making places cover both is more state for what the lock already says.
- **Never let a draft's worktree sit on a landed `impl/*` branch**, by having the draft switch back to `spec/<id>` before it lands its fix. Rejected as the fix: it narrows one path, while a build after its land and any later run that lands from its own worktree stay exposed.
