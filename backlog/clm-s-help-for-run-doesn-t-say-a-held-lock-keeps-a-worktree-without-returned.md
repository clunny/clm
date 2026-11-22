---
title: "clm's help for run doesn't say a held lock keeps a worktree without --returned"
kind: bug
components: [clm]
origin: 20261122-claude-code-keeps-a
---

The root's `Run` help in `crates/clm/src/main.rs` (the doc comment on `Command::Run`, which `clm --help` prints) says `--finish` removes the component's clean worktrees under `.claude/worktrees/` once its branch has landed. Since `orchestrator.run/run-finish@17d6dafb` a worktree whose lock a running process holds is kept unless `--returned` says the build's subagent has handed back, so the help promises a removal that doesn't happen. The change `20261122-claude-code-keeps-a` names this help as the root's to reword; `orchestrator.run`'s build couldn't, since the file is the root's. Fix: say that a worktree whose lock a running process holds is kept, unless `--returned` says the build's subagent has handed back.
