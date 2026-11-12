---
component: orchestrator.triage
---

## ADDED Requirements
### Requirement: draft-records-promised-code-work
The prompt `clm draft` writes SHALL tell the session, in the paragraph that has it design the rest as one change, that a change whose deltas add, modify and remove no requirement makes no component pending, so no build does code work its outcome or its Design edit relies on (`component-reconciliation`), and that before it applies the change it records each piece of such work: work that a requirement would have to state is added to the change as that requirement, so the change has something to build (`dependency-work-is-a-requirement`); work that changes nothing a requirement states, such as removing code nothing calls, goes into a `kind: deferred` backlog item whose `components` name the components whose `paths` hold the code and whose body names that code, which a code branch later resolves with the trailer `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`). It SHALL tell the session, in the paragraph that names the builder check, to ask the subagent too whether the change's outcome promises code work that no requirement it adds or modifies makes pending, and to record each such piece of work the same way. The text says what `plugin.propose/propose-archives-design-only` has `/clm:propose` say, so the two paths that write spec changes record promised code work alike, and keeps to the wording `draft` allows.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `gate/undeclared-burden` also contains `component-reconciliation`, `kind: deferred` and `deferred-work-resolves-in-code`, and the paragraph that contains `builder-check-asks-order` also contains `no requirement it adds or modifies makes pending`
