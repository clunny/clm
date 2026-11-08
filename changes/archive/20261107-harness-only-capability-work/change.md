---
status: done
resolves: [harness-only-capability-work-has-no-stated-route]
---

## Intent

State the route for harness-only work on a project's capability suite: work that changes only its plumbing (runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat), so no `CAPABILITIES.md` entry and nothing a capability test observes. It is never a requirement of any component. When it leaves the `capability` component's Design (the root's, before a project has that component) true as written, it takes the small-change route with no claim, and a `kind: bug` or `kind: deferred` item that names it is deleted by a commit with a `Resolves: backlog/<slug>` trailer. When the Design must change, a spec branch edits the Design with no requirement and adds a `kind: deferred` item for the code it owes, and the same session follows with the code on a small-change branch that resolves the item. `docs/spec-format.md`'s "Proving a capability", the decisions, the glossary, the propose skill (`plugin/propose-harness-only-work`, `plugin/code-fix-needs-a-written-rule`) and the draft prompt (`orchestrator.triage/draft-code-fix-written-rule`) say so. GitHub issue #268.

## Problem

`human-layer-proof` makes the harness the `capability` component's Design, which the AI owns, and that component holds no requirement of its own. Nothing tells a session what route harness-only work takes, and `code-fix-needs-a-written-rule` sends every bug no requirement states to a spec branch that adds one. One session wrote harness work as a requirement of the root component, whose build would then edit `capability`'s `paths`, which `path-ownership` forbids, and the change was parked.

## Approaches

1. **The small-change route, with the backlog recording owed work (chosen).** Harness-only work changes no rule, so it is a small change (`small-change-no-claim`); the gate already accepts a `Resolves:` trailer deleting a `bug` or `deferred` item on any branch whose code touches the item's component (`code-fix-resolves-bug`), so an item that names the work is closed without a claim. A Design change lands as a Design-only spec branch archived in its own branch (`archive-in-spec-pr`), and the `kind: deferred` item it adds records the code it owes, so nothing is lost if the session ends between the two lands (`fold-small-defer-large`). Reuses what exists (`reuse-first`).
2. **`clm claim capability --fix` for an item that names the work.** Rejected after the architecture audit: `--fix` is for a bug a requirement states and a test cites, which harness work can't have, and a harness fix that also edits another component's file (such as `.github/scripts/ci-check.sh`) would cross `path-ownership` on `impl/capability`.
3. **A requirement of `capability` for the harness.** Rejected: `capability` holds no requirements of its own by design (`human-layer-proof`), and no boundary test could cite one.
4. **A requirement of the root, with a `path-ownership` exception for its build.** Rejected: widens an ownership rule for work no requirement needs to state.
5. **A harness component of its own.** Rejected (`yagni`): a second component for a Design that already has a home.

## Outcome

- `plugin/propose-harness-only-work` (added): `skills/propose/SKILL.md` states the route.
- `plugin/code-fix-needs-a-written-rule` (modified): the skill's code-fix section names harness-only bugs as the exception.
- `orchestrator.triage/draft-code-fix-written-rule` (modified): the `clm draft` prompt fixes a harness-only item on a small-change branch in the draft, or designs the Design edit with a `deferred` item for the code.
- Hand edits: `docs/spec-format.md`'s "Proving a capability"; `docs/guide/glossary.md` gains **Small change** and **Harness** (`plain-terms`); amendments to `human-layer-proof`, `code-fix-needs-a-written-rule`, `small-change-no-claim` and `dependency-work-is-a-requirement`.

## Constraints

- `path-ownership`: no new exception; a small-change branch takes no claim, so harness work crosses no build's ownership.
- `code-fix-needs-a-written-rule`: still holds for every bug a person or test observes; only harness-only bugs, which no requirement can state, are outside it.
- `capability-tests-follow-rules` is unchanged: work that changes what a test observes is a requirement's, and its build updates the tests.
- `node-reconciliation`: a Design edit still builds nothing; the code it owes is a `deferred` item, not a queued build.

## Edge cases and workarounds

- Harness work that also touches another component's file (such as `.github/scripts/ci-check.sh`) takes the same small-change route; `clm land` checks the merged tree.
- A project with no `capability` component keeps its harness in the root's Design; the same route applies.

## Open questions

- Builder check: the requirements' scenarios check only for `harness-only`; the builder may check the route's other names (`small-change-no-claim`, `Resolves: backlog/`) as well.
