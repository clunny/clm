---
component: plugin.propose
base: { contract: "6cd7947ac2ea4e89" }
---

## MODIFIED Contract
The skill `propose`, `plugin/skills/propose/SKILL.md`: an intent to one spec branch with its pull request, landed with `clm land` after the human's approval in the session, or, for a bug whose fix changes no spec, one code branch that deletes the bug's item; it ends with `clm wrap-up` and a summary that says whether the session can be ended. The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file. The rules that ask something of this file alone are this component's, the builder check's among them (`builder-check`).

## ADDED Requirements
### Requirement: builder-check
`skills/propose/SKILL.md` SHALL tell Claude to run a builder check with a fresh subagent before opening the spec PR, to list the questions it leaves open under `## Open questions` in the PR body, to record facts and steps only a human can supply as `question` or `human-task` backlog items whose `blocks` name the requirements that wait on them, and to give a requirement that can't be built until another component's pending requirement is built a `#### Needs` block naming it (`requirement-needs`). It SHALL tell Claude that a requirement that removes or renames part of a component's Contract that another component's code uses is such a requirement: it needs the requirements that move each such component off the old contract, and a removal written only as `REMOVED Requirements` can't carry them, so it is stated as an added or modified requirement or split per `expand-contract` (`contract-removal-needs`); that a rename is written as two requirements of the provider in the same change (`rename-adds-then-removes`): an adding one that offers the new name beside the old, needs nothing and leaves every user's code building as it stands (a second name, never a widened type a user matches exhaustively), and a removing one, stated as an added or modified requirement, that needs each user's move, each move needing the adding one, so the needs form no cycle, the adding one's build marking the old name `clm:remove-by` the change, which stays live until the removal is built (`expand-contract`); and to ask the builder-check subagent to search the code of the other components for uses of each part of a Contract the change removes or renames, exhaustive ones (a `match`, a `switch` with no default, a map keyed by the type) included, and for exhaustive uses of each type the change widens. It SHALL tell Claude that no build edits another component's code or tests to follow a change (`path-ownership`), so a change that another component's code or tests must follow is written in steps, each a requirement of the component whose files it changes (`coupled-changes-sequence`): a requirement that widens a type another component matches exhaustively (an added enum variant) needs that component's requirement that first handles a value it doesn't know with a default arm marked `clm:remove-by` the change, and that component's requirement that handles the new value, removing the arm, needs the widening; a requirement that adds a field another component builds by struct literal needs that component's requirement that builds it so an added field leaves it compiling, through a constructor or defaults the provider offers first; and a requirement whose build would otherwise have to edit another component for its own tests to pass needs that component's requirement that makes the edit. It SHALL tell Claude that when the removal can't carry that `#### Needs` (the removing component is required and the component that must move is optional, or the removed text is a whole requirement), the contract step goes in a second change on the same branch, left `proposed`, and the branch also adds a `kind: deferred` backlog item for it whose `components` are the contract change's components and whose body names the contract change and each requirement it waits on with the version it must be current at, which the contract change's `resolves` doesn't name, so `clm triage` shows it until the human judges it due (`contract-step-deferred`). It SHALL tell Claude to have the builder name the work a requirement relies on from another component that no requirement of that component states, since a Contract or Design edit alone builds nothing, adding that component's requirement to the change and naming it in the relying requirement's `#### Needs` (`dependency-work-is-a-requirement`). It SHALL tell Claude to ask the builder-check subagent, for each requirement the change adds or modifies, which requirements not yet current it relies on, of any component, among the change's own and those `clm pending` lists, answering `none` for a requirement that relies on none (`builder-check-asks-order`), so a missing order shows as a named pair rather than a question the subagent may not think to ask; and that a requirement relying on a pending requirement of its own component that has a `#### Needs` block carries that block's needs too, since a need may not name its own component and the build's order within the component can't wait on another component's work.

#### Scenario: skill without the check
- GIVEN a `skills/propose/SKILL.md` whose numbered steps hold no builder-check step that names `## Open questions` and comes before the step that runs `clm pr open`
- WHEN the tests run
- THEN a test fails naming the skill

#### Scenario: skill without blocks or needs
- GIVEN a `skills/propose/SKILL.md` that doesn't mention one of `blocks` and `Needs`
- WHEN the tests run
- THEN a test fails naming the skill and the missing word

#### Scenario: skill without a sequencing rule
- GIVEN a `skills/propose/SKILL.md` that doesn't mention one of `contract-removal-needs`, `rename-adds-then-removes`, `coupled-changes-sequence` and `contract-step-deferred`
- WHEN the tests run
- THEN a test fails naming the skill and the missing name

#### Scenario: skill without dependency work
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `builds nothing`
- WHEN the tests run
- THEN a test fails naming the skill and `builds nothing`

#### Scenario: skill without the order ask
- GIVEN a `skills/propose/SKILL.md` that doesn't mention one of `builder-check-asks-order` and `its own component`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/propose-intent
- architecture/path-ownership
