---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: plan-verify <!-- base: 2e8f46fddf70434e -->
After a run of the loop landed, and when the loop becomes idle (no run running and nothing to launch), the plan SHALL run `clm verify --due --dry-run`, with `--event work-done` when idle, and, only when its stdout starts `verify due`, print `run verify clm verify --due` (with `--event work-done` when idle), also while a drain is requested. The run's outcome line, the last line of its output that starts `verify `, or `none` when no line does, SHALL be reported with `clm build returned verify --result <line>`, since `clm verify` prints notes on stderr after its outcome line (`verify-run`) and a session that runs it as background work reads both streams as one output. `clm build returned verify` SHALL exit 1, recording nothing of the return and printing no action, when `<line>` is neither `none` nor a line starting `verify `, its stderr saying to report the line of the run's output that starts `verify `, or `none`; this SHALL hold after `returned-ignores-an-earlier-task`, whose late return is ignored whatever its line. A `verify failed` line SHALL be recorded and put in the human's queue until a backlog item on `origin/main` names its commit in `verify_failures`, and SHALL NOT be drafted, since the human takes a slow test's break in (`slow-tests-after-land`).

#### Scenario: idle and due
- GIVEN nothing running or ready, and a fake `clm verify --due --dry-run --event work-done` that prints `verify due work-done` and exits 0
- WHEN `clm build step` runs
- THEN it prints `run verify clm verify --due --event work-done`

#### Scenario: red result in sight
- GIVEN `clm build returned verify --result 'verify failed 1a2b3c4 culprit 9f8e7d6 components a'`
- WHEN the plan is computed, and again after an item naming the commit in `verify_failures` lands
- THEN the first human queue holds the red result with `take in with /clm:triage`, and the second doesn't

#### Scenario: a note reported in place of the outcome
- GIVEN a running `verify`
- WHEN `clm build returned verify --result 'clm verify: output is at /tmp/v.log'` runs, and then `clm build returned verify --result 'verify failed 1a2b3c4 culprit 9f8e7d6 components a'`
- THEN the first exits 1, its stderr names the line starting `verify `, and `verify` is still running; the second exits 0, and the plan's human queue holds the red result

#### Needs
- view/queue-view
