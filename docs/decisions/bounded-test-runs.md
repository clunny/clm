---
title: "A build or draft runs its own tests and checks with a time limit, other than clm land"
date: 2026-11-09
status: accepted
---

A build started its component's tests in the background, a test hung on a polling wait with no deadline, and the run never ended. The build waited for the run's completion notice (`background-work-stopped`), the orchestrating session counted it as still running (`plugin.build/build-waiting-builds`), and nothing woke either: an idle session is woken only by a notice, and a hung command sends none. The build sat idle for about 80 minutes until the human asked.

So every test or check command a build or draft runs itself, other than `clm land`, ends within a time limit: in the foreground, where the tool call's own timeout ends it, or in the background as `timeout <limit> <command>`, the limit well past the run's expected time and chosen by the session, since a project's test time is not CLM's to know. A run its limit ends is a failing run: the session finds the hanging test as for any failure, fixes it when it is in the component it is working on, and otherwise records it as a backlog item and names it in its pull request body. The `clm run` and `clm draft` prompts say so (`orchestrator.run/bounded-test-runs`, `orchestrator.triage/bounded-test-runs`). The waiting rules stay as they are: once every command a session waits on ends by itself, waiting for its notice is safe.

The bound rests on the prompt, as `background-work-stopped` does and as `path-ownership-by-prompt` accepts for `path-ownership` (`deterministic-enforcement`): no program sees which commands a session runs or how, short of wrapping the agent's shell, and a session that ignores the instruction is no worse off than before this change. Its tests check that each prompt carries the instruction.

`clm land`, and `clm land --check-only`, stay exempt and run as written: a land queues behind other lands with no time limit (`lock-queue`), and the check command is one plain command (`prompt-check-command`). A hang inside a land's check is the land's to bound (`backlog/a-hung-land-check-holds-every-land-of-the-clone`).

Rejected:
- **The orchestrating skill checks on a build that has waited past a bound.** When every build waits, the orchestrating session is idle and nothing wakes it; it would need a timer task of its own, and when that fired it could only ask the human to kill by hand what a deadline ends by itself.
- **Requiring a deadline in the project's test runner** (`cargo-nextest`'s `slow-timeout`, for one). Not CLM's to require of an adopter, and it misses any run outside that runner.
