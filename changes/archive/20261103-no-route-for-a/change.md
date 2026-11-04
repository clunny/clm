---
status: done
resolves:
  - no-route-for-a-small-code-only-change-on-a-converged-component
---

## Intent

Say what a session does with a small code change that needs no spec change and has no `impl/*` branch to fold into: it takes no claim and lands from a branch of its own. `clm land` and the gate already allow this; nothing said so, so a session refused by `clm claim` had to guess. `CLAUDE.md` and the guide now state the route, and the refusal for a converged component names it.

## Problem

`clm claim <component>` refuses a converged component, and `clm claim <component> --fix` takes only a `kind: bug` or `kind: deferred` item, so an improvement on a converged component has no claim. The session that reported this found that `clm land` accepted a branch not named `impl/*`, and landed that way (`ci/release-input-hint`), without knowing whether that was allowed.

## Approaches

- **Unclaimed branch, stated (chosen).** A claim holds a component's place in the build queue so two builds don't race. A change with no requirement to build has no place to hold, and `clm land` already handles overlap: it merges `main` in and checks the result. `clm-gate` classifies by paths, not branch names, and `land/landed-branch-cleanup` already speaks of branches that aren't claims. Nothing needs building but the sentence in the refusal.
- **A `clm claim` form that takes no item.** Rejected. `code-fix-resolves-bug` limits `--fix` to a named item so that it can't become a general way around the queue's refusals, and such a claim would also hold the component out of the queue for nothing (`yagni`).
- **File a `kind: bug` item to get a `--fix` claim.** Rejected. It records an improvement as a defect, and a backlog item only to delete it costs a commit and protects nothing.

## Outcome

- The decision `small-change-no-claim` states the route and its limits.
- `CLAUDE.md` and `docs/guide/using-clm.md` state it. The refusal states it too, since a repository other than this one has no such line in its `CLAUDE.md`, and the refusal is where a session meets the gap (`orchestrator/claim`).
- The `--fix` route and `code-fix-needs-a-written-rule` are unchanged: a bug or deferred item still takes the claim, and behaviour no requirement states still starts on a spec branch.

## Constraints

- The route fits only a change that leaves every requirement and scenario as written true and adds nothing a requirement would have to state.
- No code changes in `clm land` or the gate.

## Edge cases

- `clm land` leaves a branch outside `impl/*`, `spec/*` and `propose/*` on `origin` (`land/landed-branch-cleanup`), so the session deletes the one it pushed once it has landed.
- A live claim on the same component isn't a bar: the two meet at `clm land` as any two branches do, and a conflict goes through `clm rebase`.
