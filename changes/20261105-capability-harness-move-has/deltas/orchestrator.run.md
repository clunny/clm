---
component: orchestrator.run
---

## ADDED Requirements

### Requirement: capability-tests-follow-rule
The `clm run` prompt SHALL tell the session that when a requirement it builds changes what a test in the `capability` component's `paths` observes at the product's boundary (a screen, a path, a message or an output the test expects), it updates that test in its own branch, since its land runs the test and the `capability` component holds no requirement that could carry the move (`path-ownership`, `capability-tests-follow-rules`): it changes only the test's expectations, and the steps that reach them, where the requirement changed them, to what the requirement states; it keeps the test's citation as it is and drops no check the capability's scenarios, as `CAPABILITIES.md` now states them, state, so when the change edited the entry the test follows the entry's current text while its citation stays at the old version, leaving the capability `behind` for its own build to re-cite (`proven-by-tests`); and it names each such fix under `## Fixes in other components` in its pull request body. It SHALL tell the session that a requirement under which a capability entry, as `CAPABILITIES.md` now states it, no longer holds is not such a fix: the session leaves the requirement unbuilt, keeping the behaviour the test checks, and adds a `question` backlog item whose `components` name both components, with `blocks` naming the requirement, and which names the entry, so a spec change edits it. The text SHALL be in every prompt, beside `removal-breaks-dependant`'s, which it doesn't change. When the repository has no `capability` component, the text SHALL still be there, since it names no path.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that a requirement that changes what a capability test observes updates that test's expectations, keeps its citation, names the fix under `## Fixes in other components`, and contains `capability-tests-follow-rules`

#### Scenario: entry no longer holds
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that when a capability entry as written no longer holds, the session leaves the requirement unbuilt and adds a `question` item with `blocks` naming the requirement and naming the entry

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
