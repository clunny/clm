---
component: plugin.agents
---

## MODIFIED Requirements
### Requirement: spec-slimmer <!-- base: 8de89fe379a4facd -->
The plugin SHALL ship `agents/spec-slimmer.md`, a subagent given a component ID whose `tools` hold neither `Edit` nor `Write`. It SHALL tell the agent to run `clm context <component>`, read the component's `SPEC.md` and the code its `paths` own, and report, each with a quote and the cut it proposes: Design text that restates a requirement or narrates what changed; a requirement that is implementation detail or duplicates another; scenarios that prove the same thing; and code no requirement needs. Before it proposes to remove or merge a requirement, it SHALL tell the agent to search for what names the requirement's ID, `<component>/<slug>`: an `#### Enforced by` block in `ARCHITECTURE.md`, a decision under `docs/decisions/`, and another component's `#### Needs` block in `specs/`; and, for each one it finds, to say so beside the proposal, quoting the reference and saying what the cut would leave unproven or waiting, since a requirement so named is how that constraint or decision is proven, or what that component waits on (`proven-by-tests`). It SHALL say the agent never edits and assigns no severity.

#### Scenario: agent that could edit
- GIVEN an `agents/spec-slimmer.md` whose `tools` include `Edit`
- WHEN the tests run
- THEN a test fails naming the agent and `Edit`

#### Scenario: agent without the context command
- GIVEN an `agents/spec-slimmer.md` that doesn't mention `clm context`
- WHEN the tests run
- THEN a test fails naming the agent and `clm context`

#### Scenario: agent that doesn't look for references
- GIVEN an `agents/spec-slimmer.md` that doesn't mention one of `#### Enforced by`, `docs/decisions/` and `#### Needs`
- WHEN the tests run
- THEN a test fails naming the agent and the missing text
