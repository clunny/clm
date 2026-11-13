---
component: orchestrator.run
base: {}
---

## ADDED Requirements
### Requirement: split-move-fix
The `clm run` prompt SHALL tell the session that when a requirement it builds was moved to its component from another component, removed there and added here with its text unchanged, as a split moves one from a component to the child it carves out (`nested-splits`), its build carries the move into the other component's `paths` (`path-ownership`, `moved-markers-wait`): it moves the code and tests that build the moved requirement out of the other component's files into its own `paths` and deletes the other's copy; it changes the other component's imports, entry module and wiring only so that they reach the moved code where it now lives, so the other component's behaviour doesn't change; and it re-points each marker of the moved requirement, wherever its file sits, to the requirement's new ID at its current version, after checking that its test proves the requirement. It SHALL tell the session that it moves nothing that builds a requirement the other component keeps, and that it names each edit to the other component's files under `## Fixes in other components` in its pull request body. The text SHALL be in every prompt, beside `removal-breaks-dependant`'s, `capability-tests-follow-rule`'s and `template-migration-fix`'s, which it doesn't change.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that a requirement moved to the component from another moves its code and tests out of the other component's files, rewires that component's imports to the moved code, re-points the moved requirement's markers to its new ID, names each edit under `## Fixes in other components`, and contains `nested-splits`

#### Serves
- architecture/path-ownership
