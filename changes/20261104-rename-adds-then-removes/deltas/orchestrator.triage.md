---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-contract-step-deferred <!-- base: 7021d512604fe650 -->
The prompt `clm draft` writes SHALL tell the session, in the paragraph that names the builder check, that a requirement removing or renaming part of a component's Contract that another component's code uses needs the requirements that move each such component off it (`contract-removal-needs`); that a rename is written as two requirements of the provider in the same change (`rename-adds-then-removes`): an adding one that offers the new name beside the old, needs nothing and leaves every user's code building as it stands (a second name, never a widened type a user matches exhaustively), and a removing one, stated as an added or modified requirement, that needs each user's move, each move needing the adding one, so the needs form no cycle; that when the removal can't carry that `#### Needs` (the removing component is required and the component that must move is optional, or the removed text is a whole requirement under `REMOVED Requirements`), the contract step goes in a second change on the same branch, left `proposed`, and the branch also adds a `kind: deferred` backlog item for it whose `components` are the contract change's components and whose body names the contract change and each requirement it waits on with the version it must be current at, which the contract change's `resolves` doesn't name, so `clm triage` prints it as a `deferred` line until the human judges it due (`contract-step-deferred`); and that a claimed `deferred` item for a contract step is designed by applying the change it names with `clm apply`, rebasing its stale deltas on `clm hashes <component>` first, the session's own change resolving the item.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `contract-removal-needs`, `rename-adds-then-removes`, `contract-step-deferred`, `optional` and `kind: deferred`
