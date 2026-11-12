---
component: plugin
---

## MODIFIED Requirements

### Requirement: skills-run-verify <!-- base: b8bb7c23040b05e4 -->
`skills/build/SKILL.md` and `skills/triage/SKILL.md` SHALL each tell Claude, at the end of the run, after its lands and before `clm wrap-up`, to run `clm verify --due --event work-done` and show the human its `verify` line; a `verify failed` line SHALL be shown as a slow test that broke on `main`, which the next triage takes in. `skills/triage/SKILL.md` SHALL also tell Claude to run `clm verify --import` just before `clm intake`, and SHALL say, among `clm intake`'s lines, that `verify <sha> new <culprit> <component>...` is a slow test that failed after the land, which no item has taken in, and that a `verify <sha> log <path>` line after it names the log this clone kept of that failed run. For each `verify <sha> new` line, Claude SHALL show the human, beside it, the failure itself when a `log` line follows it: the last 40 lines of that log and, when the check marks its steps (`check-step-markers`), the step open as the output ended, the last `::group::` line with no `::endgroup::` after it; and, when none follows, say that this clone kept no output of the run (it ran on another clone or on the host's pipeline, or its log was removed), so the decision rests on the culprit and components alone. Claude shows the lines together with no verifier, offering `P1`, and writes each one taken in as a `kind: bug` item whose `verify_failures` holds the commit's full ID (`git rev-parse <sha>`), whose `components` are the line's, and whose body names the culprit land, its change and, when a log was kept, the failing step, the lines that show the failure and the log's path, on the same `intake/<YYYYMMDD-HHMM>` branch as the other items taken in, saying that, once the item is on `main`, `clm land` holds code lands on those components until the fix, whose commit carries `Resolves: backlog/<slug>`, lands, and that an item for a failure that proves a flake is deleted on the human's word (`verify-after-land`). It SHALL say that taking a `verify` line in is the human's decision that the failure is real: until then `clm land` only warns.

#### Scenario: the shipped skills
- GIVEN the plugin's `skills/build/SKILL.md` and `skills/triage/SKILL.md`
- WHEN a test reads them
- THEN both contain `clm verify --due --event work-done`, and the triage skill contains `clm verify --import`, `verify <sha> new`, `verify <sha> log <path>`, `::group::`, `verify_failures` and `verify-after-land`

#### Scenario: verify after the lands
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN `clm verify --due --event work-done` comes before the last `clm wrap-up` in it

#### Needs
- verify/verify-due
- verify/verify-import
- orchestrator.triage/intake-verify-failures
- clm/verify-command

#### Serves
- capability/slow-tests-after-land
