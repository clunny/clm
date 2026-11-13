---
component: plugin.propose
base: {}
---

## MODIFIED Requirements

### Requirement: propose-nests-splits <!-- base: 7b38199827fc6078 -->
`skills/propose/SKILL.md` SHALL tell Claude, when a change splits a component or carves a part out of one, to make the carved-out part a child of the component it came from (`specs/<component>/<part>/SPEC.md`, ID `<component>.<part>`), to keep the product's ID on the half that stays, to move requirements by removing them from the parent and adding them unchanged to the child, and to add a requirement of the parent's, whose `#### Needs` names the moved ones, that removes the parent's copy of their code and tests and reaches the moved code through the child, since the child's build writes that code in its own files and no build edits another component's (`coupled-changes-sequence`), and to split only along architectural seams, never to satisfy a size budget (`slim-components`, `nested-splits`); to cut so that imports run one way, the child below the parent or what both halves share in a layer of its own below both (`splits-import-one-way`), which inside one crate or package means that a carved-out part using the crate's other modules goes into a crate or package of its own; to check the cut before writing a delta by running `clm survey --components <file>` on a component map of the component's halves, listing every file each will own, with `outside: ["**"]` for the rest of the code, and to change the cut until it exits 0, except that a `survey/undeclared-edge` line may stand when all three hold: the importing file moves into a crate or package of the child's own, so no file the child's `paths` match exists until the child is built; a requirement the change adds to the child, whose build writes the importing file in its own crate or package, states the edge's removal, naming the importing file, the file it imports and what replaces the import (such as a value the parent passes in), and what the parent must do for it (such as passing that value) is a requirement of the parent's whose `#### Needs` names the child's; and `change.md` lists each such edge with the child's requirement ID under `## Edges the change removes`, as the record of what the child's build must remove. A `survey/cycle` line never stands. And it SHALL tell Claude to point at `clm guide shaping-the-graph`.

#### Needs
- init/guide-command
- init/survey-components

#### Scenario: skill text
- GIVEN `plugin/skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it says a carved-out part becomes a child of its component, names `clm guide shaping-the-graph`, `clm survey --components`, `splits-import-one-way` and `## Edges the change removes`, says the parent gets a requirement that removes its copy and names `coupled-changes-sequence`, and says never to split for a budget alone

#### Serves
- capability/propose-intent
- architecture/slim-components
