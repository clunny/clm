---
title: "clm build step's wrap-up removes a landed build's worktree before the build has reported"
kind: bug
components: [orchestrator.loop, orchestrator.checkpoint]
---

What happened: during `/clm:build`, a build subagent ran `clm land` as a background command, as its prompt says, and ended its turn waiting for that command's completion notice. The land pushed `main` and the build's claim left `origin`; `clm build wait` woke ("<build>'s claim has left origin") and the next `clm build step` ran `clm wrap-up`, which removed the build's worktree under `.claude/worktrees/agent-<id>` because its branch had landed. The subagent was still live in the loop (`clm view queue` had listed it as running) and had not handed back its final report. Claude Code then refused to resume it ("its worktree no longer exists, and the fallback directory is not covered by the session's isolation fences"), so its report (review findings, mutation spot-checks, CLM defects, the PR body path) never reached the orchestrating session. It happened to four builds in one run; two others reported only because they resumed before the step ran, and both noted that their worktree had vanished under them.

Expected: a build's worktree stays until its run has handed back (the build prompt itself says the orchestrating session frees the place and tidies the worktree after reading the final line), so a landed build's report always arrives.

Rules involved: `one-report-per-run`, the loop's `build-wait` (claim left origin ends the wait before housekeeping), and the wrap-up's removal of landed build worktrees (`orchestrator.checkpoint`). The wrap-up the step runs doesn't know which worktrees belong to runs the loop still counts as running.
