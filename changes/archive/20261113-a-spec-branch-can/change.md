---
status: done
resolves:
  - a-spec-branch-can-t-remove-a-capability-entry-a-test-cites
---

## Intent

Let one spec branch remove a capability that a test cites. The change whose branch deletes the entry from `CAPABILITIES.md` lists it in `change.md` under a new key, `removes_capabilities`. While that change is live, the tests citing the capability read as `obsolete` instead of failing `clm check`, and the next build of the component that holds the capability tests deletes them, as it does for a requirement a delta removed.

## Problem

A capability is cited by its tests like a rule, but it is removed by a hand edit, not by a delta. `check/dangling-markers` exempts only a marker whose requirement a live change lists under `REMOVED Requirements`, so a branch that deletes a cited entry fails `clm check`, and it can't delete the test, which would make it `mixed`. `20261113-human-layer-rewrite` removed two entries only by landing a code branch first that deleted their tests. That takes proof off `main` before the human has agreed to the removal, and needs two lands in the right order.

The design was incomplete, not wrong: `removed-requirements-build-out` covers every removal a delta makes, and a capability has no delta. So the fix extends that mechanism to the one removal it misses instead of adding a second one.

## Approaches

- **A key in `change.md` that lists the removed capabilities** (chosen). `removed(graph)`, the one function that says what is obsolete, also gives each listed capability that `CAPABILITIES.md` no longer holds. `clm check`, `clm coverage`, `clm pending`, `clm status` and the archive step all read that function, so they follow without a rule of their own.
- **A delta for the `capability` component with `REMOVED Requirements`.** Rejected: `clm apply` would have to edit `CAPABILITIES.md`, a file people edit by hand, and `clm rebase`'s regeneration and the stale-base check would have to learn a second file. A project with no `capability` component has no component to write the delta for.
- **Reading the removal from git**, comparing with `origin/main`. Rejected: once the branch lands there is nothing to compare, and the markers would dangle on `main`.
- **Treating every marker on a missing capability as obsolete.** Rejected: a mistyped marker would never be reported.
- **Letting the spec branch delete the tests**, or keeping the code-branch-first workaround. Rejected: the first makes spec branches mixed, the second is what the item reports.

## Outcome

- `model/parse-changes`: `change.md` accepts `removes_capabilities`, a list of capability slugs, and the graph exposes it on the change.
- `check.coverage/removed-capabilities` (new): a listed capability that `CAPABILITIES.md` doesn't hold is removed while its change is outside the archive. `removed` gives it, `clm coverage` and `clm pending` print it as `obsolete`, and it is pending where `pending-report` already places an obsolete requirement: on `capability`, or without that component on the owner of each marker's file.
- `check.coverage/status-report` and `change-progress`: a change that lists a removed capability isn't complete while a marker cites it, so `clm rebase` doesn't archive the change before the build has deleted the tests.
- `check`'s Contract: its definition of `obsolete` covers a removed capability.
- `check/dangling-markers`: such a marker is no violation. The message for any other marker on a missing capability names the key, so the author of the next branch that removes a cited entry learns how.
- `check/removed-requirement-names`: a component's `SPEC.md` that still names the removed capability is reported on the spec branch, where it can be fixed.
- Decision `removed-capabilities-build-out`; `removed-requirements-build-out` amended. `docs/spec-format.md` and the glossary describe the key.

A capability test sees nothing different at the product's boundary because of these requirements until a branch uses the key, so no existing capability test changes.

## Constraints

- The entry is still removed by hand, and the branch still needs the human's yes because it edits `CAPABILITIES.md` (`removal-needs-a-yes`). The key grants no removal.
- No change can use `removes_capabilities` until `model/parse-changes` is built, since the model on `main` rejects an unknown key (`checks-relax-before-use`).
- An architecture entry needs none of this: no test cites one (`human-layer-proof`).

## Edge cases

- A listed slug that `CAPABILITIES.md` still holds, or never held, removes nothing and is no violation.
- A mistyped marker could be hidden by listing its slug. The change then can't complete, so the build removes the marker either way.
- With no `capability` component, a removed capability's markers are pending on the owner of each file, or the root, as an obsolete requirement of a removed component is. No new placing rule.

## Decisions

- `removed-capabilities-build-out`: a removed capability is built out like a removed requirement, declared in `change.md`.

## Open questions

- **No order between the capability's build and its rules' builds** (architecture audit, against `path-ownership`). A change that removes a capability often removes or changes the rules behind it. A rule's build that takes the behaviour away before the `capability` build has deleted the obsolete tests fails those tests at its land, and may not delete them. Left as a known limit in the decision: the obsolete line is never blocked and carries its change's priority, so the `capability` build is offered at once, and the rule's build goes through once it has landed. Closing it means a new kind of wait in `check.coverage/pending-report` and the queue (`orchestrator`), which is more than this bug needs before a removal has met it (`yagni`).
- **The propose skill doesn't name the key** (architecture audit, not counted as a gap). `plugin/skills/propose/SKILL.md` tells a session to remove a capability's entry by hand and says nothing of `removes_capabilities`. Left: the `check/dangling-markers` message names the key at the moment a branch needs it, and `docs/spec-format.md` and the glossary describe it, so a requirement of `plugin` for one sentence isn't worth its build.
