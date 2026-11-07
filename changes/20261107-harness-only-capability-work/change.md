---
status: applied
resolves: [harness-only-capability-work-has-no-stated-route]
---

## Intent

State the route for harness-only work on a project's capability suite: work that changes only its plumbing (runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat), so no `CAPABILITIES.md` entry and nothing a capability test observes. It is never a requirement of any component. When it leaves the `capability` component's Design (the root's, before a project has that component) true as written, it takes the small-change route, or `clm claim <component> --fix` when a `kind: bug` or `kind: deferred` item names it. When the Design must change, the Design edit lands first on a spec branch, and the same session follows with the code on a small-change branch. `docs/spec-format.md`'s "Proving a capability", `human-layer-proof` and the propose skill (`plugin/propose-harness-only-work`) say so. GitHub issue #268.

## Problem

`human-layer-proof` makes the harness the `capability` component's Design, which the AI owns, and that component holds no requirement of its own. Nothing tells a session what route harness-only work takes. One session wrote it as a requirement of the root component, whose build would then edit `capability`'s `paths`, which `path-ownership` forbids, and the change was parked.

## Approaches

1. **State the existing routes (chosen).** Harness-only work changes no rule, so it is a small change (`small-change-no-claim`) while the Design holds, and a Design-only spec branch followed by a small-change branch when it doesn't, since a Design edit alone builds nothing (`node-reconciliation`) and a change with no requirement archives in its own branch (`archive-in-spec-pr`). Reuses what already exists (`reuse-first`).
2. **A requirement of `capability` for the harness.** Rejected: `capability` holds no requirements of its own by design (`human-layer-proof`), and a harness requirement would be cited by no test at the boundary.
3. **A requirement of the root, with a `path-ownership` exception for its build.** Rejected: widens an ownership rule for work no requirement needs to state.
4. **A harness component of its own.** Rejected (`yagni`): a second component for a Design that already has a home.

## Outcome

- `plugin/propose-harness-only-work` (added): `skills/propose/SKILL.md` states the route.
- `docs/spec-format.md`'s "Proving a capability" states it (a hand edit).
- `docs/decisions/human-layer-proof.md` is amended with the route, and `docs/decisions/code-fix-needs-a-written-rule.md` with the Design as the written rule a harness fix restores, since the component that holds the harness holds no requirement that could state it.

## Constraints

- `path-ownership`: no new exception; harness work on a small-change branch takes no claim, so it crosses no build's ownership.
- `code-fix-needs-a-written-rule`: a `--fix` for a harness bug restores the Design as written; behaviour a capability test observes still needs an entry or a requirement.
- `capability-tests-follow-rules` is unchanged: work that changes what a test observes is a requirement's, and its build updates the tests.

## Edge cases and workarounds

- Harness work that also touches a file of another component (such as `.github/scripts/ci-check.sh`) still takes the small-change route: that route holds no claim, and `clm land` checks the merged tree.
