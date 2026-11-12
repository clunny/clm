---
component: plugin.triage
---

## ADDED Requirements

### Requirement: triage-verify-log
`skills/triage/SKILL.md` SHALL say, among `clm intake`'s lines, that a `verify <sha> log <path>` line after a `verify <sha> new` line names the log this clone kept of that failed run. It SHALL tell Claude, for each `verify <sha> new` line it puts to the human, to show beside it, when a `log` line follows it, the last 40 lines of that log and, when the check marks its steps (`check-step-markers`), the step open as the output ended, the last `::group::` line with no `::endgroup::` after it; and, when none follows, to say that this clone kept no output of the run (it ran on another clone or on the host's pipeline, or its log was removed), so the decision rests on the culprit and components alone. It SHALL tell Claude that the body of an item it writes for a line taken in names, when a log was kept, the failing step, the lines that show the failure and the log's path, beside the culprit land and its change (`verify-after-land`).

#### Scenario: the shipped skill
- GIVEN the plugin's `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN it contains `verify <sha> log <path>`, `last 40 lines`, `::group::`, `::endgroup::` and `kept no output`

#### Needs
- orchestrator.triage/intake-verify-failures

#### Serves
- capability/slow-tests-after-land
