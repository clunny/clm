---
status: done
resolves:
- a-test-plumbing-fix-can-t-delete-a-bug-item-that-names-another
---

## Intent

Tell a draft what to do when a test-plumbing fix lies under a component the item doesn't name: correct the item's `components` and leave the fix to the next draft, as it already does for a code-only bug filed on the wrong component.

## Problem

The draft prompt's test-plumbing route has the session fix the plumbing on a branch of its own and delete the item in a commit with the trailer `Resolves: backlog/<slug>`, landing once `clm gate --base origin/main` prints `code` (`draft-code-fix-written-rule`, `small-change-no-claim`). The gate passes that deletion only when a code file the branch changes belongs to a component the item at the merge base names (`gate/backlog-additions`). `backlog/flaky-panel-test-fails-a-land-removing-its-temp-directory` named `plugin.panel`, its fix lay under `capabilities/panel/`, which is `capability`'s, and the gate printed `mixed`. The prompt gives a components correction only for a code-only bug fixed by `clm claim --fix` (`draft-corrects-bug-components`) and says nothing for test plumbing, so the draft had to work the route out. `human-layer-proof`'s amendment says the gate accepts the trailer "on any branch", which is true of the branch's name and reads as if the item's components didn't matter.

## Approaches

- **(a) The prompt gives test plumbing the correction it gives a code-only bug (chosen).** One modified requirement, `orchestrator.triage/draft-corrects-bug-components`: the session sets the item's `components` on `spec/<id>` to the components the fix's files belong to, leaves the item with `left <slug>: components corrected to <components>` and opens no branch for it. The next `clm triage` groups it by its corrected components and that draft fixes it. One rule for both kinds of wrongly filed item, and the triage skill already reads that `left` line.
- **(b) The same draft lands the correction and then the fix.** It saves a draft session, and it is what the session that reported this did. The prompt would have to order a land of `spec/<id>` before a fix branch cut from the new `origin/main`, the reverse of every other route in it, where code fixes land first and the spec branch last. Not chosen: a wrongly filed plumbing item is rare, and a second ordering in the prompt costs every draft.
- **(c) The gate passes the deletion for test plumbing whatever component the item names.** The gate can't tell test plumbing from other code: that is reading, not a program's (`deterministic-enforcement`). Dropping the component bound for every trailer would let any code branch delete any bug or deferred item, which `code-fix-resolves-bug` bounds on purpose. Not chosen.

## Outcome

- `orchestrator.triage/draft-corrects-bug-components`: the prompt's code-only paragraph says a `bug` or `deferred` item whose work is test plumbing, and whose fix changes only files belonging to no component the item names, has its `components` corrected on `spec/<id>` and is left, with the reason: the gate passes a trailer's deletion only for an item naming a component the branch's code belongs to, and otherwise prints `mixed`. A file belongs to a component whose `paths` hold it or whose requirement it cites, as the gate counts it (`fixes-count-where-cited`).
- `docs/decisions/small-change-no-claim.md` gains an amendment saying the same, and that "on any branch" in `human-layer-proof` speaks of the branch's name; `docs/decisions/human-layer-proof.md` gains a line saying so too, and the glossary's **Test plumbing** entry states the bound, so an attended session reads it in the guide (`method-in-the-plugin`).

## Constraints

- Build order (builder check): `draft-corrects-bug-components` relies on no requirement that isn't current. Its new sentence goes in the prompt paragraph that already holds `components corrected to <components>`, worded as the exception to the branch of its own, and the phrases the new scenario names appear in no other paragraph. `scenario_draft_corrects_bug_components` is re-cited at the new version.

- The Design of `orchestrator.triage` (**The job**) already says an item filed on the wrong component is corrected and cites this requirement; it stays true as written and is not edited.
- No capability test sees anything different: the prompt's text is checked by `orchestrator.triage`'s own tests.
- Edge cases and workarounds known: none.

## Open questions

- The architecture audit found that `plugin/skills/propose/SKILL.md` and `docs/guide/how-it-works.md` state the test-plumbing route without the gate's bound (`method-in-the-plugin`). Left: the glossary entry now states it for attended sessions, and a `plugin.propose` requirement would add a build for a case met once. Worth a requirement if an attended session meets the `mixed` gate.
- The audit found that `plugin/skills/triage/SKILL.md` calls a `left <slug>: components corrected to <components>` line "a bug the draft found filed on the wrong component", and the line can now come from a `deferred` plumbing item. Left: what the skill does with the line is the same for either kind, and one word doesn't earn a `plugin.triage` build.
