---
component: plugin
---

## ADDED Requirements

### Requirement: propose-harness-only-work
`skills/propose/SKILL.md` SHALL tell Claude that harness-only work, which changes only the plumbing of the project's capability suite (its runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat) and so changes no `CAPABILITIES.md` entry and nothing a capability test observes, is never a requirement of any component: the harness is the Design of the component that builds the capability tests, `capability` or, until a project has it, the root (`human-layer-proof`). It SHALL say that when that Design stays true as written, the work takes the small-change route (`small-change-no-claim`), or `clm claim <component> --fix backlog/<slug>` when a `kind: bug` or `kind: deferred` item names it; and that when the Design must change, the Design edit lands first on a spec branch with no requirement, archived in that branch (`archive-in-spec-pr`), and the same session follows with the code on a small-change branch from `origin/main` once it has landed, since a Design edit alone builds nothing (`node-reconciliation`).

#### Scenario: skill without the harness route
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Serves
- capability/propose-intent
- architecture/path-ownership
