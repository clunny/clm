---
component: plugin.propose
---

## MODIFIED Requirements

### Requirement: propose-claims <!-- base: 63373bf050b66f3e -->
`skills/propose/SKILL.md` SHALL tell Claude to run `clm proposals` and leave out the items it names before suggesting or picking backlog items; to claim each backlog item the intent comes from with `clm claim backlog/<slug>` before brainstorming, and to stop and tell the human when a claim is refused; to cut the spec branch, write `change.md` with `from` or `resolves`, commit and push it right after claiming; to keep a claimed item that design finds already fixed in `resolves`, deleting its `backlog/<slug>.md` on the spec branch and naming in `## Intent` what fixed it; and, once it knows the change's components and again before opening the PR, to run `clm proposals --component` for them and ask the human what to do when it prints a `proposed` or `claimed` line. It SHALL tell Claude that a `main` line, a change already applied on `main`, needs no question. It SHALL write that command as `clm proposals --component <component> [--component <component>]...`, with `--component` once per component, as the CLI takes it, and never as `--component <component>...`, which reads as one flag followed by several components.

#### Scenario: skill without claims
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm claim backlog/`
- WHEN the tests run
- THEN a test fails naming the skill and `clm claim backlog/`

#### Scenario: skill without the overlap check
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `clm proposals --component`
- WHEN the tests run
- THEN a test fails naming the skill and `clm proposals --component`

#### Scenario: one flag for several nodes
- GIVEN a `skills/propose/SKILL.md` that writes `clm proposals --component <component>...`
- WHEN the tests run
- THEN a test fails naming the skill and the form `clm proposals --component <component> [--component <component>]...`

#### Scenario: skill without already-fixed items
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `already fixed`
- WHEN the tests run
- THEN a test fails naming the skill and `already fixed`

#### Scenario: asks about applied changes
- GIVEN a `skills/propose/SKILL.md` that doesn't mention a `main` line
- WHEN the tests run
- THEN a test fails naming the skill and `main` line

#### Serves
- capability/work-concurrently
- capability/keep-a-backlog
