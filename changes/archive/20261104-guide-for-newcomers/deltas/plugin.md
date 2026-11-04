---
component: plugin
---

## MODIFIED Requirements

### Requirement: method-pointers <!-- base: 0c098706dc28eacc -->
`skills/init/SKILL.md` SHALL tell Claude, before drafting the component map, for a new project and for a repository with code, to explain to the human that a component is the unit one build session holds, its spec and its code; that a component splits along seams by nesting the carved-out part as its child; that plumbing several components copy becomes a shared component; and that a component over its size budget is a visible state, not an error; to point at `clm guide shaping-the-graph`; and, in its closing summary, to point at `clm guide using-clm` for the workflows and at `clm guide how-it-works` for how a change flows and where decisions go. No skill or agent SHALL name `docs/guide/`. Every `clm guide <page>` a skill or agent names, `<page>` being a lowercase name of letters, digits and hyphens, SHALL be a page `clm guide` prints: a file `docs/guide/<page>.md`, `docs/spec-format.md` for `spec-format`, or `docs/guide/README.md` for `index`.

#### Needs
- init/guide-command

#### Scenario: init explains the shape
- GIVEN an `init` `SKILL.md` that doesn't mention `clm guide shaping-the-graph`, or whose sentence naming it doesn't come before the component map is drafted
- WHEN the tests run
- THEN a test fails naming the skill and `clm guide shaping-the-graph`

#### Scenario: init without the flow pointer
- GIVEN an `init` `SKILL.md` that doesn't mention `clm guide using-clm`
- WHEN the tests run
- THEN a test fails naming the skill and `clm guide using-clm`

#### Scenario: init without the mechanisms pointer
- GIVEN an `init` `SKILL.md` that doesn't mention `clm guide how-it-works`
- WHEN the tests run
- THEN a test fails naming the skill and `clm guide how-it-works`

#### Scenario: a repository path
- GIVEN a skill that names `docs/guide/shaping-the-graph.md`
- WHEN the tests run
- THEN a test fails naming the skill and `docs/guide/`

#### Scenario: a page that doesn't exist
- GIVEN a skill that names `clm guide no-such-page`
- WHEN the tests run
- THEN a test fails naming the skill and `no-such-page`
