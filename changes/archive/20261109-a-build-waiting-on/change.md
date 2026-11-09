---
status: done
resolves: [a-build-waiting-on-a-hung-background-command-waits-forever]
---

## Intent

A build or draft that waits on a hung test run of its own waits forever, and so does the session that orchestrates it. This change has the `clm run` and `clm draft` prompts tell the session that every test or check command it runs itself, other than `clm land`, ends within a time limit: in the foreground, where the tool call's own timeout ends it, or in the background under `timeout <limit> <command>`. A run its limit ends is a failing run, whose hanging test the session finds as for any failure. Decision: `bounded-test-runs`. Resolves `backlog/a-build-waiting-on-a-hung-background-command-waits-forever`.

## Problem

During `/clm:build`, a build started its component's tests as a background command, twice. A test hung (a polling wait with no deadline), so neither run ended. The build waited for the runs' completion notices, as `orchestrator.run/background-work-stopped` tells it to, and reported that it was waiting on its own background work; `plugin.build/build-waiting-builds` then has the orchestrating session count it as still running. Nothing in that chain has a clock: an idle session is woken only by a notice, and a hung command sends none. The build sat idle for about 80 minutes until the human asked; the session then found and killed the hung processes by hand.

The design fault is the unbounded command, not the waiting rule: every wait the rules prescribe is for a command to end, so a command that can hang must carry its own end. `clm draft`'s prompt has the same rule (`orchestrator.triage/background-work-stopped`) and its code-fix path runs the repository's checks, so the change covers both prompts, as `one-report-per-run` did.

## Approaches

- **The prompt bounds the session's own test runs (chosen).** The fix sits where the hang starts and needs no clock anywhere else: the run ends, its notice arrives, and the build reads a failure it can act on. It costs one paragraph of prompt per prompt (`yagni`), and the foreground's tool-call timeout and `timeout(1)` are tools the session already has (`reuse-first`).
- **The build skill checks on a build that has waited past a bound.** Rejected: the orchestrating session is idle when every build waits, so nothing wakes it to check; it would need a timer task of its own, and when it fired it could only ask the human to do by hand what the deadline does by itself. It treats the symptom one level up.
- **A deadline in the project's test runner** (such as `cargo-nextest`'s `slow-timeout`). Not CLM's to require of an adopter, and it wouldn't cover a run outside that runner; this repository may add one as test plumbing on its own.

## Outcome

- `orchestrator.run/bounded-test-runs`: the build prompt's waiting paragraph tells the session to bound each test or check command it runs itself, other than `clm land` (and `clm land --check-only` when `[land] check` is set), says why, and says a run its limit ends is a failing run.
- `orchestrator.triage/bounded-test-runs`: the same for the draft prompt, other than `clm land` in any form.
- `plugin` changes nothing, though the item names it: its waiting rules (`plugin.build/build-waiting-builds`, `plugin.triage/triage-waiting-drafts`) stay right once every command a build or draft waits on, except a land, ends by itself.

## Constraints

- `clm land` and `clm land --check-only` stay exempt and run as written: a land queues behind other lands with no time limit (`background-work-stopped`, `lock-queue`), and the check command is one plain command (`prompt-check-command`). A hang inside a land's check is the land's to bound; it is recorded as `backlog/a-hung-land-check-holds-every-land-of-the-clone`.
- The text keeps to the wording each prompt's other requirements allow: no `cd`, no exit code, no `clm watch`, and `clm land --check-only` named in the build prompt only when `clm.toml` sets `[land] check`.
- The limit's value is the session's to choose, well past the run's expected time; the prompt names none, since a project's test time is not CLM's to know.

## Edge cases and workarounds

- A run that is slow rather than hung and passes its limit counts as failing; the session reads which test was running and runs it again with a longer limit. Accepted: a false alarm costs a rerun, a missed hang costs the whole build.
- A hang inside `clm land` or `clm land --check-only` still waits with no limit, as above.

## Open questions

None.
