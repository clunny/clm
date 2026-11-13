---
description: Build the approved specs (not compile) by running the clone's build loop with clm build, component by component and in parallel, until it is drained or stopped. Use when the user wants approved specs built; /clm:build runs the unattended builds, it does not compile.
---

# Build approved changes

Run the clone's build loop. `clm build` decides every launch, order, retry, resume, fold, archive and verify; you carry out the actions it prints with the agent harness's subagents, report what comes back, and talk to the human (`build-loop-in-clm`, `agent-harness-adapters`, `builds-in-subagents`).

Before starting, tell the human once that runs launch under this session's permission mode: in manual mode they pause for the human's answers, while auto or bypass mode lets them run alone. Never refuse a mode.

Once, before the first launch (`unattended-permissions-check`), run `clm init --check-permissions`. When it exits 1, tell the human that in auto mode Claude Code may deny the unattended steps those rules allow, show the rules it printed, say that they belong in `permissions.allow` in `.claude/settings.json` (or `.claude/settings.local.json` for this clone alone), and go on launching, never refusing to. When it exits 0, go on. When it exits with any other code, as a `clm` without the option does, tell the human the check couldn't run and go on launching. Never write either file yourself.

Run `clm agent-harness` first (`skills-follow-agent-harness`): it says what this session's agent harness can do. Where it can't stop a subagent, a `stop` action waits for the run to return instead (below).

Start the loop with `clm build start --session-model <alias>`, `<alias>` the alias of this session's own model, the one your system prompt names (such as `opus`). Then carry out each action every `clm build` command prints, in order. Load a deferred tool with `ToolSearch` first.

- `launch`: start a background subagent with the Agent tool, as a `general-purpose` agent with `isolation: "worktree"` and the line's model, its prompt the named file's text unchanged. Then run `clm build launched <name> <task-id>`. When the line says to run it in the session, do its work here instead.
- `resume`: send the file's text with `SendMessage`. When it can't be delivered, report it with `clm build returned <name> --result none --cause tool`.
- `stop`: stop the run with `TaskStop`. Where the agent harness can't stop a subagent, wait for it to return and report it with `--cause stopped`.
- `run`: run the command as a background Bash command, and report its last line with `clm build returned`.
- `wait`: run `clm build wait` as a background Bash command.
- `say`: show the file's text to the human once.
- `end`: end the loop.

Report every subagent's or background command's return with `clm build returned <name> --result <last line>`, the last line as it stands. When `clm build wait` exits, run `clm build step`.

For a return that isn't `landed <sha>`, `proposed <id> review <url>` or `waiting: <command>`, pass with `--cause` your reading of why the run ended. It is your one judgement; `clm build` keeps the counts and decides what follows from it:

- `limit`: a usage or rate limit error; add `--until <time>` when the error says when it lifts.
- `disk`: a full disk.
- `outage`: a network outage.
- `tool`: a tool or host failure.
- `work`: a check or test that fails, or an approach that went wrong.
- `context`: a run that ran out of context.
- `blocked`: a run that stopped on a backlog item or a spec change.
- `stopped`: a run the loop told it to stop.

When the agent harness still lists a run as running background work after a `landed <sha>` line, report it with `--cause running`.

Never decide a launch, an order, a retry or a resume yourself. Never do a run's work in your own context unless the `launch` line says to run it in the session. Never run a command aimed at a run's worktree, such as `git -C <worktree>`.

When `clm build start` refuses because another session holds the loop, show the human `clm view queue`, and run `clm build start --take-over` only when the human says that session is gone. When the human asks to drain or stop the loop, run `clm build stop --drain` or `clm build stop --now`. When the human says they freed the disk, run `clm build cleared disk`. When the human gives up a component's build for good, release its claim by deleting `impl/<component>` on `origin` with `clm branch delete`.

What needs the human is in the loop's queue, which `clm view queue` and the CLM panel's Queue tab show until it is resolved: point the human at it rather than keep a question, a stop or a condition only in the chat, and when the human asks what is waiting on them, answer from `clm view queue`.

When the queue holds a fold batch waiting for approval, ask the human once, in this conversation, whether they approve the intent of every folded change (`approve-in-session`), showing the pull request's link and each change's `## Intent`, `## Decisions` and `## Open questions`. On a yes, run `clm approve spec/<batch>`, which the guard has them confirm, and then `clm build approved <batch>`. Never approve or land a change otherwise.

Report each CLM defect you meet with `clm defect --title <title> --component <component>` when it happens, naming with `--component` the component a fix would lie in, the report on stdin describing only the process, never the project's files, code, specs or output (`defect-reports`). You open no pull request of your own, so the loop drafts reported defects with the backlog's other bugs.

Never push `main` by any means but `clm land`. Run `clm approve` only after the human says yes in this conversation, and only through the guard's confirmation: never route around the guard or ask another tool to do it. Only a human approves a spec change (`humans-review-intent`).
