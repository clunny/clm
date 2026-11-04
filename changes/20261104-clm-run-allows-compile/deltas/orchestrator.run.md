---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: removal-breaks-dependant <!-- base: 85d13797646fefd9 -->
The `clm run` prompt SHALL tell the session never to edit another component's code or tests to absorb what its own component removes from or adds to its Contract, except for the compile-only and test-fixture fixes the removal or the addition forces (`architecture/path-ownership`, `forced-fixes-cover-additions`): where the other component's code or tests only name what is removed (a `match` arm on a removed variant, a fixture that sets a removed field, an assertion on a removed field or state) and nothing that component's code does depends on it, or only build a value that gains a field (a struct literal, in code or in a fixture, that must now name the added field) and the fix sets that field to the value under which the provider behaves as before, so the other component's behaviour doesn't change, the session SHALL make the fix and name each one under `## Fixes in other components` in its pull request body. It SHALL say that a rename is not such a fix (`rename-adds-then-removes`), and neither is an arm for an added variant that another component matches exhaustively, nor an added field with no value under which the provider behaves as before, since those decide what the other component does. When building a requirement would break another component's code or tests because that component still uses what the requirement removes, or because what it adds needs more there than such a fix (a rename, an arm for an added variant or a field with no such value), the session SHALL keep the old path working and that component building as it stands, leave the requirement unbuilt, and add a `question` backlog item whose `components` name both components, with `blocks` naming the requirement and saying which component uses what; for an `obsolete` requirement it SHALL remove the markers and tests, keep the code the other component uses, with no `clm:remove-by` marker, and add a `deferred` item without `blocks`, whose `components` name both components and whose body names the kept code, the component that uses it, and that the code goes once that component no longer uses it (`contract-removal-needs`, `deferred-work-resolves-in-code`). The text SHALL be in every prompt, beside `pending-prompt`'s instruction to remove an obsolete requirement's code, which it qualifies and leaves in place.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says not to edit another component to absorb a removal or an addition except for the compile-only and test-fixture fixes it forces, named under `## Fixes in other components`, to keep the old path, and to add a `question` item with `blocks` naming the requirement

#### Scenario: addition forces a fix
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says a struct literal in another component that must name an added field is fixed by setting the field to the value under which the provider behaves as before, and that a rename, an arm for an added variant or a field with no such value is not such a fix

#### Scenario: obsolete requirement
- GIVEN the prompt `clm run` builds for a component with no `obsolete` requirement
- WHEN the tests read it
- THEN it says to keep code another component still uses and to add a `deferred` item without `blocks`

#### Serves
- architecture/path-ownership
- capability/no-dead-weight
