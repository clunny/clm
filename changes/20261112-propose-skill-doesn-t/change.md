---
status: applied
resolves: [propose-skill-doesn-t-tell-an-archive-in-spec-change-to-record]
---

## Intent

Have `/clm:propose` record the code work a change with nothing to build relies on. A change whose deltas add, modify and remove no requirement is archived in its own spec branch (`archive-in-spec-pr`) and makes no component pending, so no build ever does code work its outcome promises. Before archiving such a change, the skill now has Claude add a requirement for work a requirement would have to state, or file a `kind: deferred` item, naming the code, for work that changes nothing a requirement states (such as removing code nothing calls); and the builder check asks whether the outcome promises code work that no requirement the change adds or modifies makes pending.

## Problem

`20261111-slim-flagged-specs` filed this. An earlier Design-only change said in its Outcome that "the build removes" some code nothing calls. No requirement changed, so no component went pending, no build ran, and the code stayed (`node-reconciliation`: a Design edit alone builds nothing). Nothing recorded the promised work, and it surfaced only when a later slimming pass found the same code. The skill's Harness-only section already tells a Design edit to add a `kind: deferred` item for the code it owes; step 9's archive paragraph, which every other design-only change goes through, says nothing of the kind, and the builder check asks only about work another component's requirement must state (`dependency-work-is-a-requirement`), not about work the change itself promises.

## Approaches

1. **Say it where the change is archived, and ask it in the builder check (chosen).** The archive paragraph is the one place every design-only change passes; there Claude records each piece of promised code work, as a requirement when one would have to state it, or as a `kind: deferred` item otherwise, which a code branch resolves with `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`), as the harness route already does. The builder check, which reads the change fresh, is where an outcome promising code nothing builds shows. Both reuse rules that exist (`reuse-first`).
2. **Have a design-only change make its components pending.** Rejected: `node-reconciliation` rejected that, since a component's pending state would no longer be read from markers alone and nothing would say when a Design edit is built; `dependency-work-is-a-requirement` keeps that answer.
3. **A `clm check` rule that flags an archived change whose text promises a build.** Rejected (`yagni`): it would need to read intent from prose, the heuristic `dependency-work-is-a-requirement` rejected; the builder check reads the change as a builder would, which is where the gap shows.

## Outcome

`plugin.propose/propose-archives-design-only` gains the record step in the paragraph that names `clm archive` and a builder-check point, with a scenario whose test fails when either is missing. Its build edits `plugin/skills/propose/SKILL.md` and extends `design_only_gaps` and `propose_archives_design_only_changes` in `crates/clm/tests/plugin_propose.rs`, re-citing the test. No capability test observes the skill's text. Nothing else changes: the `clm draft` prompt has no design-only archive step, so it has no such paragraph to extend.

## Constraints

- `node-reconciliation`: a Design edit builds nothing, which is why the promised work must be a requirement or an item.
- `dependency-work-is-a-requirement`: work a requirement would have to state becomes one, and the change is then not design-only.
- `deferred-work-resolves-in-code`: the deferred item is closed by the code branch that does the work, with the trailer.
- `fold-small-defer-large`: the item is how the deferred work stays visible.

## Open questions

- The prompt `clm draft` prints (`orchestrator.triage`) archives no design-only change and asks its builder check nothing about promised code work either; a design-only draft's outcome could make the same promise. Left out of this change, which the item scopes to `plugin.propose`; worth a backlog item if it recurs.
