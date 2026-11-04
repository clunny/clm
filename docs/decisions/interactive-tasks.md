---
title: "Tasks that write Claude Code's protected paths are built interactively"
date: 2026-10-22
status: superseded
superseded_by: builds-in-subagents
---

A task is `interactive` when its delta names a path Claude Code protects (`.claude/`, `.git/`, `.npmrc`, `.mcp.json` and the rest of its documented list) that falls under the task's node's `paths`. The queue derives this each time it is read. `clm next`, and so `implement`, skips such a task, and `clm run` refuses it. `clm claim` takes it, and a human has it built in an interactive session, confirming each protected write when Claude Code asks. Claude still writes the file (`ai-writes-code`).

**Why.** Under `dontAsk`, Claude Code denies writes to protected paths before it reads any allow rule, so no setting lets an unattended session make them. Twice a plan handed a session such a write (`.claude/settings.json`, `web/.npmrc`), and the session blocked partway on a `human` item. Knowing up front is cheaper, and the check is deterministic (`deterministic-enforcement`).

**Matching.** A backticked path in the delta counts only when it falls under the node's `paths`, so a crate's Design that mentions `.git/info/exclude` doesn't make its task interactive. The rule can still over-flag, for example a Design that names a protected file it only reads. That costs the human a session, never a broken build. A path it misses falls back to the session blocking on a `human` item.

**Alternatives.** `bypassPermissions` would let sessions write these files, but it drops every other check for every task. A plan-written `human` item per write would need a new way for a plan to block a task, would rest on the model's judgment, and would have the human edit files by hand. A gate check after the fact comes too late, because the session was already refused. Writing a derived field into `plan.md` gives the same behaviour as deriving it in the queue, but it changes the plan format and breaks existing plans' checks.
