---
component: upgrade
---

## MODIFIED Requirements
### Requirement: migration-offer-verify <!-- base: 8280ddabd302cd84 -->
An `advised` apply migration SHALL bring a project set up before `clm verify` the commented-out `[verify]` section `clm init` writes (`init/init-verify-comment`), appending it to `clm.toml`, after one blank line, when it holds no line that is `[verify]` or `# [verify]` ignoring surrounding whitespace. Its `done` SHALL be true when `clm.toml` holds such a line or doesn't exist. Its description's first line SHALL read `Slow tests can run after the land: offer [verify]`, so `/clm:upgrade`, seeing it applied, asks the human whether to set it (`plugin.upgrade/upgrade-skill-offers-verify`). It changes no setting: the section stays commented until the human sets it.

#### Scenario: offered to an older project
- GIVEN a `clm.toml` with no `[verify]` line
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists the migration as `advised apply Slow tests can run after the land: offer [verify]`, the second prints `applied` with its ID, and `clm.toml` then ends with the commented section and loads with no `verify` setting

#### Scenario: done once set or commented
- GIVEN a `clm.toml` holding the line `# [verify]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command
- init/init-verify-comment

#### Serves
- capability/slow-tests-after-land
