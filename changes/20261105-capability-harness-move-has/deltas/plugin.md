---
component: plugin
---

## ADDED Requirements

### Requirement: builder-check-capability-tests
`skills/propose/SKILL.md` SHALL tell Claude, among the builder check's points, to ask the builder-check subagent which capability tests each requirement the change adds or modifies changes what they observe at the product's boundary, and that such work is neither a requirement of `capability`, which holds none of its own, nor a `#### Needs`, nor a backlog item: the requirement's own build updates those tests (`capability-tests-follow-rules`), so Claude names them in that requirement's text; and that a requirement under which a `CAPABILITIES.md` entry as written no longer holds edits that entry in the change instead.

#### Scenario: skill without the capability tests
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `capability-tests-follow-rules`
- WHEN the tests run
- THEN a test fails naming the skill and `capability-tests-follow-rules`

#### Serves
- capability/propose-intent
- architecture/path-ownership
