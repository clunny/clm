---
title: "implement watches for new work in the session through a blocking clm next --wait"
date: 2026-11-01
status: superseded
superseded_by: build-loop-in-clm
---

When `/clm:implement` has drained the queue and finished its wrap-up (defect fixes, archiving, `clm checkpoint`, the summary), it offers to watch for new work. On a yes it runs `clm next --wait` as a background command and, when that exits with work, starts again at its first step. After each later drain it wraps up again and goes back to waiting without asking, until the human says to stop (`20261101-implement-watches`).

**Waiting is a CLI form, not prose.** `clm next --wait [--interval <minutes>]` looks as `clm next` does and, while it would print nothing, waits and looks again, fetching each time, until it would print a node. The poll's behaviour (the interval, a failing fetch, the first look's exit codes) is then code that tests prove (`deterministic-enforcement`), and the same as `clm runner`'s: a default of 5 minutes, and a failure after the first look printed and waited out rather than ending the wait. Run as a background command, it costs the session no turns while nothing happens, and Claude Code wakes the session when it exits.

**Wrap up before each wait.** Step 5 runs at every drain, before the wait, so everything is saved on `origin` and the summary is current whenever the human looks; the session can still be ended or cleared at any point of the wait.

**For the session only.** The watch is never a setting: it ends when the human says stop, clears or ends the session. A standing preference would start every implement session waiting, which the human hasn't asked for, and `clm runner` already serves watching with no session open (`machine-runners`).

**Alternatives.** A shell loop in the skill (`until [ -n "$(clm next)" ]; do sleep 300; done`) needs no code, but its behaviour lives only in prose and a failing `clm next` spins silently or wakes the session for nothing. Claude Code's `/loop` with timed wakeups spends a turn per look, and only the human can start it, so the skill could neither offer it nor keep it on. The name `clm watch` is not reused: skills may not name it (`implement-subagents`), as it was an earlier command's.

**Superseded 2026-11-12** (`build-loop-in-clm`). The build loop is continuous: `clm build wait` replaces `clm next --wait`, waking on everything a step would launch while runs still go, and no wrap-up precedes it.
