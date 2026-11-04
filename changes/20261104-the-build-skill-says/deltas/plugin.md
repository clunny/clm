---
component: plugin
---

## ADDED Requirements

### Requirement: build-says-what-is-ready
`skills/build/SKILL.md` SHALL say, where it tells Claude to run `clm next`, that `clm next` prints the components that have a pending requirement, are unclaimed, and have a pending requirement that no `#### Needs` and no blocking backlog item holds back, and that a component's `depends_on` doesn't order builds, naming `needs-alone-order` and `component-reconciliation`. It SHALL NOT say that `clm next` waits for a component's dependencies to be settled or built, since the queue doesn't (`orchestrator/next`), so a session following the skill never waits on, or explains the queue by, an order CLM doesn't apply.

#### Scenario: the shipped build skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `needs-alone-order`, `component-reconciliation`, `#### Needs` and `depends_on`, and doesn't contain `dependencies are all settled`

#### Scenario: a build skill that orders by dependencies
- GIVEN a `build` `SKILL.md` that says `clm next` prints the components whose dependencies are all settled
- WHEN the test's check runs on it
- THEN it reports `dependencies are all settled`

#### Serves
- capability/build-from-spec
