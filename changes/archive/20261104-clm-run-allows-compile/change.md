---
status: done
resolves: [clm-run-allows-compile-only-fixes-in-other-components-only-for]
---

# A build may make the compile-only fixes an addition forces

## Intent

A build that adds a field to a public struct of its component breaks every struct literal of that struct in another component's code and tests, and nothing lets it fix them: `architecture/path-ownership` and `clm run`'s prompt (`orchestrator.run/removal-breaks-dependant`) allow the compile-only and test-fixture fixes in other components only for a removal. This change widens that exception to an addition on the same terms: the other component only builds the value, and the fix sets the new field to the value under which the provider behaves as before, so that component's behaviour doesn't change. The build names each fix under `## Fixes in other components`, as for a removal. A rename, an arm for an added variant another component matches exhaustively, and an added field with no such value stay outside it. Decision: `forced-fixes-cover-additions`.

## Problem

The builder check of `20261104-detectors-skip-in-the-root` found that adding a field to a library crate's struct broke the root binary's struct literals the moment the field existed. A `#### Needs` can't order the users' fix, since it needs the field first, and the provider's build may not edit the users, so the change had to design around it. The limit is written in two places: `ARCHITECTURE.md`'s `path-ownership`, whose first exception names only a removal, and `removal-breaks-dependant`.

## Approaches

- **Widen the build's exception to additions whose fix changes no behaviour (chosen).** The build is the one that sees the compile error, the fix is mechanical, and the review and spec audit already read fixes named in the pull request body (`path-ownership-by-prompt`).
- **Have the propose builder check name the pattern, so the change states the fix.** Rejected: every such change would repeat the same sentence, and a change whose builder check missed a literal still leaves its build stuck.
- **Make public structs non-exhaustive or give them builders.** Rejected: a design rule on every library crate for a one-line fix, and `#[non_exhaustive]` forbids struct literals outside the crate, tests included.
- **Two changes, the field first behind a hidden default.** Rejected: two reviews and a deferred step for a fix that changes no behaviour.

## Outcome

- `ARCHITECTURE.md`'s `path-ownership` names an addition beside a removal in its first exception and gains the scenario `addition forces a fix` (a hand edit, reviewed with the change).
- `orchestrator.run/removal-breaks-dependant` (modified) has the prompt allow the fixes an addition forces, set to the value under which the provider behaves as before, and say what is not such a fix; it keeps its slug, since the removal half is unchanged and `path-ownership`'s `#### Enforced by` names it.
- `docs/decisions/forced-fixes-cover-additions.md`, with amendments on `contract-removal-needs`, `path-ownership-by-prompt` and `rename-adds-then-removes`.

## Constraints

- `rename-adds-then-removes` keeps its route: a rename's users still move by their own requirements. Its adding requirement may rely on the struct-literal fix when it adds a field, which an amendment there says (from the architecture audit).
- A build that meets a rename, an added variant's arm or a field with no behaviour-preserving value leaves the requirement unbuilt and files a `question` item with `blocks`, as for a removal another component still uses (made explicit after the builder check).
- The added-variant case stays a design matter: the adding requirement must not widen a type a user matches exhaustively.

## Edge cases and workarounds

- A field whose every value changes what the provider does has no behaviour-preserving value: the build leaves the requirement unbuilt and files a `question` item, as for a removal another component still uses.

## Open questions

None yet.
