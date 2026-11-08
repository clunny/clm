---
component: plugin
---

## MODIFIED Requirements

### Requirement: code-fix-needs-a-written-rule <!-- base: ea8812db5c78fd48 -->
`skills/propose/SKILL.md` SHALL tell Claude that a bug takes the code-branch route (`propose-code-fixes`) only when a requirement or scenario as written already states the behaviour the fix restores, and the fix's test cites that requirement; when none does, the fix starts with a spec branch that adds the requirement or scenario, and the code fix follows on `impl/<component>` citing it, since a fix that no requirement states is code that serves nothing written, which the next rewrite, slimming or `no-dead-code` pass can remove unseen (`code-fix-needs-a-written-rule`). It SHALL say, in the same place, that an `ARCHITECTURE.md` entry is not such a requirement, since a test never cites an entry (`human-layer-proof`), so a bug whose behaviour only an architecture entry states is a bug no requirement states; and that a bug whose work is test plumbing only is the exception, never getting a requirement: a bug in the capability suite's harness alone is harness-only work (`propose-harness-only-work`), and a bug in the plumbing of a component's own test suite (its runner settings, the ports and servers its tests start, the paths they write), changing nothing any of its tests asserts, takes the same route, its test citing no requirement.

#### Scenario: skill without the rule
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `code-fix-needs-a-written-rule`, or doesn't contain `as written`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Scenario: skill without the harness exception
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `harness-only`
- WHEN the tests run
- THEN a test fails naming the skill and `harness-only`

#### Scenario: skill without the test-plumbing exception
- GIVEN a `skills/propose/SKILL.md` whose code-fix section doesn't contain `own test suite`, or doesn't contain `only an architecture entry states`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/propose-intent
- architecture/fold-small-defer-large
