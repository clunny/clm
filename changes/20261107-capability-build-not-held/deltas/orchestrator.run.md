---
component: orchestrator.run
---

## ADDED Requirements

### Requirement: capability-waits-for-changed-rule
The `clm run` prompt for the component that builds the capability tests (`prompt-serves`) SHALL tell the session that when a capability's test can't pass because a rule it exercises is still `behind`, its rebuild not yet landed, the session stops at that capability and leaves it pending: it files no backlog item and re-cites no test whose checks still prove the entry's old text (`proven-by-tests`), since `clm pending` holds a capability back while such a rule is blocked and, when nothing blocks the rule, the rule's build is ready and the capability's next build finds it current (`serving-rules-order-capabilities`). A prompt for any other component SHALL NOT hold this text.

#### Scenario: capability tests component
- GIVEN the prompt `clm run` builds for the component that builds the capability tests
- WHEN the tests read it
- THEN it says that a capability whose test can't pass because a rule it exercises is still `behind` is left pending, with no backlog item filed, and it contains `serving-rules-order-capabilities`

#### Scenario: other component
- GIVEN the prompt `clm run` builds for a component that doesn't build the capability tests
- WHEN the tests read it
- THEN it holds no such text

#### Serves
- capability/build-from-spec
- capability/prove-by-tests
