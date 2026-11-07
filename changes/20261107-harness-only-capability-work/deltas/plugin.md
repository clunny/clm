---
component: plugin
---

## ADDED Requirements

### Requirement: propose-harness-only-work
`skills/propose/SKILL.md` SHALL tell Claude that harness-only work, which changes only the plumbing of the project's capability suite (its runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat) and so changes no `CAPABILITIES.md` entry and nothing a capability test observes, is never a requirement of any component: the harness is the Design of the component that builds the capability tests, `capability` or, until a project has it, the root (`human-layer-proof`). It SHALL say that when that Design stays true as written, the work takes the small-change route (`small-change-no-claim`) with no claim, even when it touches another component's files, and that when a `kind: bug` or `kind: deferred` item names the work, the commit that deletes the item holds the trailer `Resolves: backlog/<slug>` (`code-fix-resolves-bug`), never a `clm claim --fix`, since no requirement states the harness for the fix's test to cite. It SHALL say that when the Design must change, a spec branch edits the Design with no requirement, is archived in that branch (`archive-in-spec-pr`), and adds a `kind: deferred` item whose `components` name the Design's component and whose body names the code the edit owes, since a Design edit alone builds nothing (`node-reconciliation`); and that once that branch has landed, the same session does the code on a small-change branch from `origin/main` that resolves the item with that trailer (`deferred-work-resolves-in-code`).

#### Scenario: skill without the harness route
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Serves
- capability/propose-intent
- architecture/path-ownership

## MODIFIED Requirements

### Requirement: code-fix-needs-a-written-rule <!-- base: 35aca90881f85240 -->
`skills/propose/SKILL.md` SHALL tell Claude that a bug takes the code-branch route (`propose-code-fixes`) only when a requirement or scenario as written already states the behaviour the fix restores, and the fix's test cites that requirement; when none does, the fix starts with a spec branch that adds the requirement or scenario, and the code fix follows on `impl/<component>` citing it, since a fix that no requirement states is code that serves nothing written, which the next rewrite, slimming or `no-dead-code` pass can remove unseen (`code-fix-needs-a-written-rule`). It SHALL say, in the same place, that a bug in the capability suite's harness alone is the exception: it is harness-only work (`propose-harness-only-work`) and never gets a requirement.

#### Scenario: skill without the rule
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `code-fix-needs-a-written-rule`, or doesn't contain `as written`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Scenario: skill without the harness exception
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Serves
- capability/propose-intent
- architecture/fold-small-defer-large
