---
component: plugin
---

## ADDED Requirements

### Requirement: init-skill-asks-verify
`skills/init/SKILL.md` SHALL, once `clm init` has written `clm.toml`, tell Claude to ask the human whether the project has tests too slow to run on every land, and on a yes, which command runs them, where they run (`local` on the developers' machines, `pipeline` in the host's pipeline, which then records the result itself and needs the right to push `clm/ledger/*`, or `import`, where the pipeline only reports and `clm verify` reads its runs), and which triggers to use (`change-done`, `queue-drained`, `work-done`, `every`), recommending `change-done`; then to replace the commented `[verify]` section with the settings, keep those tests out of `[land] check`, and, for `pipeline` and `import`, say what the host's pipeline must run, naming `clm guide ci-and-deployment`. On a no it SHALL leave the section commented. It SHALL name `verify-after-land`.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `[verify]`, `local`, `pipeline`, `import`, `change-done`, `work-done`, `ci-and-deployment` and `verify-after-land`

#### Needs
- init/init-verify-comment
- verify/verify-due
- clm/verify-command

#### Serves
- capability/slow-tests-after-land

### Requirement: skills-run-verify
`skills/build/SKILL.md` and `skills/triage/SKILL.md` SHALL each tell Claude, at the end of the run, after its lands and before `clm wrap-up`, to run `clm verify --due --event work-done` and show the human its `verify` line; a `verify failed` line SHALL be shown as a slow test that broke on `main`, which the next triage takes in. `skills/triage/SKILL.md` SHALL also tell Claude to run `clm verify --import` just before `clm intake`, and SHALL say, among `clm intake`'s lines, that `verify <sha> new <culprit> <component>...` is a slow test that failed after the land, which no item has taken in: Claude shows them together with no verifier, offering `P1`, and writes each one taken in as a `kind: bug` item whose `verify_failures` holds the commit's full ID (`git rev-parse <sha>`), whose `components` are the line's, and whose body names the culprit land, its change and the verify log, on the same `intake/<YYYYMMDD-HHMM>` branch as the other items taken in, saying that, once the item is on `main`, `clm land` holds code lands on those components until the fix, whose commit carries `Resolves: backlog/<slug>`, lands, and that an item for a failure that proves a flake is deleted on the human's word (`verify-after-land`). It SHALL say that taking a `verify` line in is the human's decision that the failure is real: until then `clm land` only warns.

#### Scenario: the shipped skills
- GIVEN the plugin's `skills/build/SKILL.md` and `skills/triage/SKILL.md`
- WHEN a test reads them
- THEN both contain `clm verify --due --event work-done`, and the triage skill contains `clm verify --import`, `verify <sha> new`, `verify_failures` and `verify-after-land`

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

### Requirement: upgrade-skill-offers-verify
`skills/upgrade/SKILL.md` SHALL tell Claude, when `clm upgrade --apply` prints `applied` for the migration that adds the commented `[verify]` section (`upgrade/migration-offer-verify`), to ask the human the same questions `/clm:init` asks about slow tests (`init-skill-asks-verify`), set `[verify]` on a yes, and leave the section commented on a no.

#### Scenario: the shipped upgrade skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN a test reads it
- THEN it contains `[verify]` and `Slow tests can run after the land`

#### Needs
- upgrade/migration-offer-verify
- clm/verify-command

#### Serves
- capability/slow-tests-after-land
