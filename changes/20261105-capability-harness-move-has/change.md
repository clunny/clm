---
status: applied
resolves: [capability-harness-move-has-no-route]
---

# A rule's build updates the capability tests it changes

## Intent

When a requirement of a product component changes what a capability test observes at the boundary, such as the app's opening screen, the build of that requirement updates the test in its own branch: `path-ownership` gains a third exception, and `clm run`'s prompt says so. The fix changes only what the test expects, and the steps that reach it, to what the requirement states, keeps the test's citation and every check its capability's scenarios state, and is named under `## Fixes in other components`. At design time the propose skill's builder check and the `clm draft` and `clm draft --fold` prompts ask which capability tests each requirement changes, and the change names them in the requirement's text, as `web/app-at-app-path` already does; that work is neither a `capability` requirement, a `#### Needs` nor a backlog item. A requirement under which a `CAPABILITIES.md` entry no longer holds edits the entry instead. Decision: `capability-tests-follow-rules`.

## Problem

- The `capability` component holds no requirements of its own (`human-layer-proof`), so `dependency-work-is-a-requirement` gives no requirement to add for moving its tests, and a `#### Needs` has nothing to name.
- The queue builds the capability after the rule (`serving-rules-order-capabilities`), but the rule's land runs the whole suite, the old capability tests included, which then fail; and `path-ownership` forbids the rule's build from editing them.
- A project's draft improvised a `deferred` item fixed through `clm claim capability --fix`, which no prompt describes, and which can't land after the rule since the rule's land already fails.
- `web/app-at-app-path` and `web.worker`'s route move already wrote the move into their own requirement text, case by case, citing `path-ownership` loosely: the route exists in practice, unstated as a rule.

## Approaches

1. **The rule's build updates the capability tests it changes, as a third `path-ownership` exception (chosen).** The move and the rule must land together, since each fails without the other; the build is the one that sees the failing test; and it generalises what `web/app-at-app-path` already does (`reuse-first`). The design-time naming lets the reviewer see the move with the intent; the build's rule catches what the builder check missed, as `forced-fixes-cover-additions` found for struct literals.
2. **A requirement of `capability` for the move, with a `#### Needs` on it.** Rejected: the moved test fails until the rule is built and the rule's land fails until the test moves, so the needs would form a cycle; and `capability` holds no requirements by design.
3. **A `deferred` item fixed with `clm claim capability --fix`.** Rejected: the same split across two lands, `main` red in between, and a `#### Needs` can't name an item (`requirement-needs`).
4. **Leave it to each change's requirement text.** Rejected: a change whose builder check missed a test leaves its build stuck.

## Outcome

- `ARCHITECTURE.md`'s `path-ownership` names the third exception, gains the scenario `rule changes what a capability test sees`, and lists `orchestrator.run/capability-tests-follow-rule` under `#### Enforced by` (a hand edit, reviewed with the change).
- `orchestrator.run/capability-tests-follow-rule` (added): `clm run`'s prompt states the fix and its limits, and that an entry that no longer holds is a `question` item.
- `orchestrator.triage/draft-capability-test-fixes` (added): the builder-check paragraph of the `clm draft` and `clm draft --fold` prompts names it.
- `plugin/builder-check-capability-tests` (added): `skills/propose/SKILL.md`'s builder check names it.
- `capability`'s Design says how its tests move with a rule.
- `docs/decisions/capability-tests-follow-rules.md`, with amendments on `path-ownership-by-prompt` and `dependency-work-is-a-requirement`.
- `model` is among the item's components but needs no change: no requirement of the model rejects the route, and `#### Needs` keeps naming only requirements.

## Constraints

- `path-ownership`: the exception is narrow, to the test's expectations and the steps that reach them, and each fix is named in the pull request, read by the build's code review and spec audit (`path-ownership-by-prompt`).
- `requirement-needs`: no `#### Needs` names a backlog item.
- `draft-checks-hand-edits`: a draft carries no code, so nothing here asks a draft to file and fix an item in one session.
- `proven-by-tests`: a fix keeps the checks the entry states now, so a test never stays current-cited while proving a text the entry no longer has (from the architecture audit).
- `fold-small-defer-large`: a named fix under a `path-ownership` exception isn't out-of-scope work (from the architecture audit).

## Edge cases and workarounds

- A requirement under which a capability entry as written no longer holds: the change edits `CAPABILITIES.md`, the rule's build moves the tests to the entry's new text with their citations at the old version, and the capability, `behind`, re-cites them in its own build. A build that finds such a case the change missed leaves the requirement unbuilt and files a `question` item with `blocks` naming it (corrected after the architecture audit).

## Open questions

- The architecture audit noted that `forced-fixes-cover-additions` rejected design-time naming while this change adds it: kept, since this fix changes what a person sees, and the decision says why.
