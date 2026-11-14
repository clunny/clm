---
title: "A build or draft runs every command of its own with a time limit, other than clm land"
date: 2026-11-09
status: accepted
---

Two builds hung on commands that never ended. One started its component's tests in the background, a test hung on a polling wait with no deadline, and the run never ended: the build waited for the run's completion notice, the orchestrating session counted it as still running, and nothing woke either, since an idle session is woken only by a notice and a hung command sends none. It sat idle for about 80 minutes until the human asked. The other hung for over five hours on a background command whose first part was a stray `cat >> <file>` ahead of its edit and test run: `cat` waited for standard input that nothing would ever type, so the test run behind it never started and a limit on that run alone would never have applied (`20261112-a-build-hung-for`).

So every command a build or draft runs itself, other than `clm land`, ends within a time limit, its test and check runs and every other command alike:
- In the foreground, the tool call's own timeout ends it.
- In the background, it runs as `timeout <limit> <command>`, each command of a chain included. For a test or check run the limit is well past the run's expected time and chosen by the session, since a project's test time is not CLM's to know.
- No command it runs reads standard input that nothing feeds it; it writes a file with its editing tools instead. A command given its input by a heredoc or a pipe, such as `clm defect` or `clm workaround` with its report on stdin, is fine, since its input ends.

A test or check run its limit ends is a failing run: the session finds the hanging test as for any failure, fixes it when it is in the component it is working on, and otherwise records it as a backlog item and names it in its pull request body. The `clm run` and `clm draft` prompts say all of this. The waiting rules stay as they are: once every command a session waits on ends by itself, waiting for its notice is safe.

The bound rests on the prompt, as the rule that a build stops its background work does and as `path-ownership-by-prompt` accepts for `path-ownership` (`deterministic-enforcement`): no program sees which commands a session runs or how, short of wrapping the agent's shell, and a session that ignores the instruction is no worse off than without it. The instruction is prompt source that no test pins (`prompt-text-is-source`).

`clm land`, and `clm land --check-only`, stay exempt and run as written: a land queues behind other lands with no time limit (`lock-queue`), and the check command is one plain command (`prompt-check-command`). A hang inside a land's check is the land's to bound (`backlog/a-hung-land-check-holds-every-land-of-the-clone`).

Rejected:
- **The orchestrating skill checks on a build that has waited past a bound.** When every build waits, the orchestrating session is idle and nothing wakes it; it would need a timer task of its own, and when that fired it could only ask the human to kill by hand what a deadline ends by itself.
- **Requiring a deadline in the project's test runner** (`cargo-nextest`'s `slow-timeout`, for one). Not CLM's to require of an adopter, and it misses any run outside that runner.
