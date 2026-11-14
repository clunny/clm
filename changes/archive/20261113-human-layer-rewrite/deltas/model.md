---
component: model
---

## MODIFIED Requirements

### Requirement: requirement-needs <!-- base: 0d58e0b7cef70490 -->
A requirement MAY hold one `#### Needs` block among its scenarios: a bullet list of requirement IDs `<component>/<slug>`, the requirements of other components that must be current before it can be built (`requirement-needs`). The model SHALL expose each requirement's needs. The block runs from its heading to the next `###` or `####` heading; blank lines in it are ignored, and headings inside code fences don't count, as for scenarios. The model SHALL report, as violations that keep the component loaded (as `dependency-dag` does): a need naming no existing requirement, so the spec PR that removes a requirement edits what needed it; a need naming a human-layer entry (`human-layers`), which isn't built in order with rules; a need naming a requirement of the same component, since order within a component is the build's; a need from a required component's requirement to an optional component's (`optional-nodes`), so CLM still works without its optional parts; a malformed block (a line that isn't a `- <component>/<slug>` bullet, a second `#### Needs` in one requirement, or one outside any requirement); and a cycle of needs, naming the requirements on it. Needs of different requirements of two components in opposite directions are not a cycle. The block is part of the requirement's text, so adding or removing a need changes its version (`content-hash`).

#### Scenario: need parsed
- GIVEN `specs/a/SPEC.md` whose requirement `x` has a `#### Needs` block with `- b/y`, and `specs/b/SPEC.md` with requirement `y`
- WHEN the model loads `specs/`
- THEN `a/x` needs `b/y`, and loading reports nothing

#### Scenario: dangling need
- GIVEN `a/x` with `#### Needs` naming `b/ghost`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `b/ghost`

#### Scenario: need on a human-layer entry
- GIVEN `a/x` with `#### Needs` naming `architecture/git-is-truth`, an entry of `ARCHITECTURE.md`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `architecture/git-is-truth`

#### Scenario: same node
- GIVEN `a/x` with `#### Needs` naming `a/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md` and `a/y`

#### Scenario: cycle
- GIVEN `a/x` needing `b/y` and `b/y` needing `a/x`
- WHEN the model loads `specs/`
- THEN loading reports `a/x` and `b/y`

#### Scenario: opposite directions
- GIVEN `a/x` needing `b/y` and `b/z` needing `a/w`
- WHEN the model loads `specs/`
- THEN both graphs load and loading reports nothing

#### Scenario: required needs optional
- GIVEN component `b` with `optional: true`, component `a` without it, and `a/x` needing `b/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `b/y`

#### Scenario: malformed block
- GIVEN `a/x` with a `#### Needs` block whose line is `needs b/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md` and `a/x`

#### Serves
- capability/build-from-spec
- architecture/component-reconciliation
