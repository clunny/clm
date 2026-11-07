---
component: upgrade
---

## ADDED Requirements

### Requirement: migration-offer-verify
An `advised` note migration SHALL offer a project set up before `clm verify` the choice of running its slow tests after the land. Its `done` SHALL be true when `clm.toml` holds a line that is `[verify]` or `# [verify]`, ignoring surrounding whitespace, or doesn't exist; it SHALL have no `old` or `new`. Its note's first line SHALL read `Slow tests can run after the land: offer [verify]`, and its body SHALL tell the session to ask the human, as `/clm:init` asks, whether the project has tests too slow for every land; on a yes, which tests, where they run (`local`, `pipeline` or `import`) and which triggers to use, writing `[verify]` into `clm.toml` and, for `pipeline` and `import`, saying what the host's pipeline must run (`clm guide ci-and-deployment`); on a no, to run `clm upgrade --decline` with its ID, so the slow tests stay in `[land] check`.

#### Scenario: offered to an older project
- GIVEN a `clm.toml` with no `[verify]` line
- WHEN `clm upgrade` runs
- THEN it lists the migration as `advised note Slow tests can run after the land: offer [verify]`

#### Scenario: done once set or commented
- GIVEN a `clm.toml` holding the line `# [verify]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command

#### Serves
- capability/slow-tests-after-land
