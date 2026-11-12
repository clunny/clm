---
status: applied
resolves: [a-build-hung-for-hours-on-a-background-command-of-its-own-and]
---

## Intent

Bound every command a build or draft runs itself, not only its test and check runs. The `clm run` and `clm draft` prompts (`bounded-test-runs`) already have a session run its tests and checks in the foreground or under `timeout <limit> <command>`. They now say the same of any command the session starts in the background, each command of a chain included, and that no command it runs reads standard input, since nothing ever types into one: the session writes a file with its editing tools instead. Resolves `backlog/a-build-hung-for-hours-on-a-background-command-of-its-own-and`.

## Problem

During `/clm:build`, a build started a background command whose first part was a stray `cat >> <file>` ahead of its edit and test run. `cat` waited for standard input for over five hours. The build waited for the command's completion notice (`background-work-stopped`), and the orchestrating session counted it as still running (`plugin.build/build-waiting-builds`), so nothing happened until the human asked. `bounded-test-runs` was built by then, but it covers only test and check commands. The command that hung was neither, and since it ran ahead of the test run, the test run's limit never applied.

## Approaches

1. **Widen `bounded-test-runs` to every command the session runs itself, and forbid reading standard input** (chosen). This is the same mechanism and the same place in each prompt, so it is a wording change to two built requirements. The decision `bounded-test-runs` already says what makes waiting safe: once every command a session waits on ends by itself, waiting for its notice is safe. The fix restores that premise.
2. **Have `/clm:build` look at a build that has waited past a bound and has pushed nothing, and tell the human** (the item's second suggestion). The decision `bounded-test-runs` already rejected this, and that reasoning still holds. When every build waits, the orchestrating session is idle and only a notice wakes it. It would need a timer task of its own, and when that fired it could only ask the human to kill by hand what a deadline ends by itself. `plugin.build` therefore gains nothing. That part of the item is accepted as it stands.
3. **Run background commands with standard input closed by some wrapper of CLM's.** No program of CLM's sees the commands a session runs (`deterministic-enforcement`, `path-ownership-by-prompt`), so this would mean wrapping the agent's shell. Rejected for that reason.

## Outcome

- `orchestrator.run/bounded-test-runs` and `orchestrator.triage/bounded-test-runs` (modified). Each prompt now says the following:
  - every command the session runs itself ends within a time limit, other than `clm land` (and, in `clm run`, `clm land --check-only` when `[land] check` is set);
  - a command it starts in the background runs as `timeout <limit> <command>`, whatever it is, and so does each command of a chain;
  - no command it runs reads standard input, such as `cat` with no file or a `cat >> <file>` meant to write one, and it writes files with its editing tools instead;
  - a command that never ends by itself, such as a hanging test or one waiting on standard input, leaves the session and its orchestrator idle with no one told.

  Only a test or check run that its limit ends counts as a failing run, whose hanging test the session finds. The scenarios also check for `standard input` and `editing tools`.
- `docs/decisions/bounded-test-runs.md` records this second incident and the widened bound.

No capability test observes these prompts' wording. There are no edge cases or workarounds.

## The human's answers

None of the triage session's answers bears on this item.

## Constraints

- `deterministic-enforcement`: the bound rests on the prompt, as `background-work-stopped` does, and its tests check that each prompt carries the instruction.
- `yagni`: no watchdog in the orchestrating skill (Approaches 2).
