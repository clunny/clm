---
component: plugin.propose
---

## MODIFIED Requirements
### Requirement: propose-archives-design-only <!-- base: 1cb1a00ec3a3ba40 -->
`skills/propose/SKILL.md` SHALL tell Claude that when the change's deltas add, modify and remove no requirement, so it has nothing to build (`archive-in-spec-pr`), it archives it as the last commit before opening the pull request, after the builder check: it sets the change's `status: done`, runs `clm archive <id>`, deletes each `backlog/<slug>.md` the change's `resolves` names, and runs `clm check` again, so the change lands on `main` already archived. It SHALL NOT tell Claude to resolve a conflict under `specs/` by hand on such a branch, since `clm rebase` regenerates `specs/` for an archived change as for an applied one (`sync/reapply`). It SHALL tell Claude, in the paragraph that names `clm archive`, that such a change makes no component pending, so no build does code work its outcome or its Design edit relies on (`node-reconciliation`), and that before archiving it records each piece of such work: work that a requirement would have to state is added to the change as that requirement, so the change has something to build and is not archived this way (`dependency-work-is-a-requirement`); work that changes nothing a requirement states, such as removing code nothing calls, goes into a `kind: deferred` backlog item whose `components` name the components whose `paths` hold the code and whose body names that code, which a code branch later resolves with the trailer `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`). It SHALL tell Claude, among the builder check's points, to ask the subagent whether the change's outcome promises code work that no requirement it adds or modifies makes pending.

#### Scenario: skill without archiving
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm archive`
- WHEN the tests run
- THEN a test fails naming the skill and `clm archive`

#### Scenario: skill with archiving
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it names `clm archive`, `status: done` and `archive-in-spec-pr`, and no sentence of the paragraph that names `archive-in-spec-pr` both names `specs/` and says `by hand` (step 8's general rule for conflicted files elsewhere stays)

#### Scenario: skill without the promised work recorded
- GIVEN a `skills/propose/SKILL.md` whose paragraph naming `clm archive` doesn't contain `kind: deferred` or `deferred-work-resolves-in-code`, or whose builder-check step doesn't contain `no requirement it adds or modifies makes pending`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/wrap-up-session
- capability/propose-intent

#### Needs
- clm/renamed-commands
- sync/reapply
