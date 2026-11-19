---
title: "A subagent's work is counted twice once its worktree is removed or reused"
kind: bug
components: [effort]
---

`effort/transcripts` counts a subagent's entries for the branch checked out in the linked worktree their `cwd` is under, as `git worktree list` names it when the record is made, and for the entries' own `gitBranch` when that worktree is detached or gone. Since a record never shrinks (`record-command`), the same entries move between records over time:

- A build run as a subagent in `.claude/worktrees/agent-x` lands, recording its work for `impl/a`. The worktree is then removed. The next `clm effort record`, or the launching session's own land through `unrecorded`, reads those entries for their `gitBranch` (say `main` or `spec/x`), whose group grows past the fold, so the tokens are recorded under both branches.
- A worktree path reused for another branch (`clm-loop` reuses `.claude/worktrees/agent-<name>`) moves the old subagent's work onto the new branch, again counting it twice.

The Design accepts double counting only for a worktree still detached when a record is made; removal happens after every build, so every build run as a subagent is charged twice: once to its `impl/<component>` and once to the launching session's branch (usually unattributed). A fix needs a spec change, for instance recording the branch a subagent file counted for when first recorded, or only counting a subagent file whose worktree is gone when no record of the session holds it under a worktree branch. Found by the code review of the `effort/transcripts@3669b654` build.
