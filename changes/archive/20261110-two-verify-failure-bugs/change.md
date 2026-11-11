---
status: done
from: backlog/two-verify-failure-bugs-on-one-component-block-each-other-s-fixes
resolves:
  - two-verify-failure-bugs-on-one-component-block-each-other-s-fixes
---

## Intent

Let separate fixes for two `verify_failures` bugs that hold the same component each land. `land/verify-stop` admits a code branch on a held component once a commit it adds resolves at least one item holding that component, instead of every such item; the items it doesn't resolve stay on the backlog and go on holding the component. The refusal names every item holding each component it refuses, and says one of them is enough. `land`'s Contract sentence on the stop says the same, and the decision `verify-stop-admits-each-fix` records the choice against `verify-after-land`.

## Problem

When two `kind: bug` items with `verify_failures` both name component `b`, a branch that fixes one item and carries `Resolves:` for it alone is refused, because the other item still holds `b`; a branch fixing the other item is refused for the same reason. Neither lands, two drafts working the items apart wait on each other until they give up, and the only way out is to merge both fixes onto one branch by hand. `land/verify-stop` has no scenario with two items on one component, so the rule as written (resolve "every item holding a component it touches") was never weighed for that case.

## Approaches

1. **Admit a branch that resolves at least one item holding each held component it touches.** Chosen. The stop keeps a component whose slow tests a human judged broken closed to all but the work that repairs them (`verify-after-land`); a fix for one of its items is that work. The stop never judged a fix's contents beyond the trailer (a branch resolving the only holding item may carry any other code on the component), so admitting a fix of one of several items lets in nothing new, and each item still holds the component until its own fix lands.
2. **Only make the refusal name every holding item and say one branch must resolve them all.** Rejected: the separate fixes still can't land, and triage's per-group drafts still deadlock.
3. **Triage puts every `verify_failures` bug on a component into one draft** (`orchestrator.triage`, `plugin.triage`). Rejected (`yagni`): items that share one component but differ in others still split, it ties unrelated failures' drafts together to dodge a land rule, and a fix made by hand for one item would still be refused.

## Outcome

- `land/verify-stop` (modified): a touched held component is *fixed* when a commit of the branch resolves at least one item holding it; the branch lands when every held component it touches is fixed. The refusal names each held component it touches and isn't fixed, with every item that holds it, and says that one item holding each named component is enough. New scenarios: *two items hold one component* (refused with both named), *each fix of two lands* (a fix of one lands; the other item still holds the component), and *each held component needs its own fix* (resolving `b`'s item doesn't admit `c`).
- `land`'s Contract: the sentence on the stop says a branch lands once a commit resolves that item or another item holding the same component.
- `verify`'s Design, **What it stops**: the hold it describes lets through, meanwhile, a branch resolving another item holding the same component.
- Decision `docs/decisions/verify-stop-admits-each-fix.md`, refining `verify-after-land`, which now names it.
- Code (built by `land`): in `crates/clm-land/src/verify_stop.rs`, compute blocking per touched held component (blocked when no item holding it is resolved), list every holding item in the refusal and reword its last line; tests for the new scenarios in `crates/clm-land/tests/verify_stop.rs`, and the requirement's other tests re-cited at its new version.

## Constraints

- No edge cases or workarounds are known.
- The capability `slow-tests-after-land` still holds as written: code lands on a broken component are refused until a fix lands, and the capability scenario has one break.

## Open questions

- The architecture audit noted that `plugin`'s `/clm:verify` requirement (and `plugin/skills/verify/SKILL.md` step 5) tells the human that once the item is on `main`, `clm land` holds code lands on its components until a fix resolving it lands. Left unchanged: it describes the hold of the item being taken in, which stays true, and never claimed that nothing else on the component may land; rewording it would cost a `plugin` build for no change in what the human must do. Say so if you want it to mention fixes of other items too.
