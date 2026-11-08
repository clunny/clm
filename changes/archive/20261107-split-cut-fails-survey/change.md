---
status: done
resolves: [split-cut-fails-survey-on-an-edge-the-change-removes]
---

## Intent

Let a split's cut keep an import edge that the change's own requirements remove, instead of pushing the session towards a worse cut. `propose` checks a cut with `clm survey --components` against the tree as it stands and changes the cut until the survey exits 0, so an edge that one of the change's requirements takes away (a carved-out file importing a parent module that the requirement replaces with a value the parent passes in) still fails it, and neither the skill nor `shaping-the-graph` says what to do (GitHub issue #245). This change amends `splits-import-one-way` and `propose-nests-splits` to say when such a `survey/undeclared-edge` line may stand, and how the change shows it to the reviewer.

## Problem

The survey reads the tree before any file moves, which is what makes it a cheap check: whether files can move into a crate of their own is whether their imports already run one way. But a split often comes with requirements that change the carved-out code's boundary, and the survey can't see those. The rule "change the cut until it exits 0" then pushes the session to move the parent's module into the child, or to give up a cut along the right seam.

## Approaches

- **A field in the survey's component map listing the excused edges, each with the requirement that removes it, which the survey would then not report.** Rejected: it adds a key and code to `clm survey` (component `init`) for a judgement the survey can't make, since the requirement it would name lives in a delta the survey doesn't read, and `clm check` already proves after the build that no such edge is left (`reuse-first`, `yagni`).
- **The skill and guide say when the line may stand, and the change lists the edges for the reviewer (chosen).** No code changes beyond the skill's text. The conditions: the importing file moves into a crate or package of the child's own; a requirement the change adds to the child, whose build moves the importing file (`path-ownership`), states the removal, naming the importing file, the file it imports and what replaces the import, and what the parent must do for it, such as passing the value in, is a parent's requirement whose `#### Needs` names the child's; and `change.md` lists each such edge with the child's requirement ID under `## Edges the change removes`, as the record of what the child's build must remove. A `survey/cycle` line never stands.

## Outcome

- `propose-nests-splits` (plugin) says when a `survey/undeclared-edge` line may stand, and its scenario checks the skill names `## Edges the change removes`. Its build edits `skills/propose/SKILL.md` and the test `propose_nests_splits`.
- `docs/decisions/splits-import-one-way.md` is amended with the rule and the rejected map field; `docs/guide/shaping-the-graph.md` says the same for adopters (hand edits).

## Constraints

- The move into a crate of the child's own is what keeps `clm check` green on the spec branch: until the build makes the child's files, no file its `paths` match exists, so `check/undeclared-imports` (and its before-apply judging) has nothing of the child's to judge. An in-place cut's edge would fail `clm check` on the spec branch before any build could remove it, so it never stands.
- Nothing new enforces the removal: once the child is built, `clm check` reports any edge its build left as `check/undeclared-imports`, or the `depends_on` cycle it would need, so the graph stays acyclic after the build.
- `## Edges the change removes` is a record, not a review gate: a split usually lands on checks, and the approval ask doesn't show the section. What proves the removal is `clm check` after the child's build (architecture audit finding, settled this way).
- Edge cases and workarounds known now: none.

## Open questions

- `triage-checks-splits` still offers a cut only when the survey exits 0. A triage recommends a cut before any change exists, so it has no requirement to name; such a cut is left for `/clm:propose` with the human, who can state the removal. Widen it only if triage sessions are seen dropping right cuts for this reason.
