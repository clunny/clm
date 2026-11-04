---
status: applied
resolves: [no-stated-route-for-a-guide-only-edit-that-needs-no-change]
---

# A stated route for a guide-only or decision-only edit

## Intent

Say what a session does with an edit that changes only `docs/` or `CLAUDE.md` and no requirement, scenario or design, such as rewording or adding a guide page, or recording a decision already made. It takes the small-change route (`small-change-no-claim`) with no change: a branch of its own named outside `impl/`, `spec/`, `propose/` and `autosync/`, whose gate prints `spec` rather than `code` and which lands on checks. `clm land` and the gate already allow this, and sessions have landed that way (`guide-newcomer`, `file/decisions-written-while-private`), but nothing said so: `CLAUDE.md`'s small-change route required the gate to print `code`, and the spec-branch route assumes a change with deltas. `CLAUDE.md`, `docs/guide/how-it-works.md` and the decision `small-change-no-claim` now state it.

## Problem

`docs/` and `CLAUDE.md` are spec paths (`gate/classify`), so a branch that edits only a guide page gates `spec`, not `code`, and fails the small-change route as `CLAUDE.md` wrote it. A `spec/<change-id>` branch carries a `change.md` and deltas, which a wording edit doesn't have. So a session had to guess that a plain branch with no change, gating `spec` with no review, may land.

## Approaches

1. **Extend the small-change route to doc-only edits (chosen).** Same branch naming, same steps, same branch deletion after the land; only the gate's first line differs. Nothing in `clm land` or the gate changes: `land/spec-branch-holds-spec-paths` binds only `spec/*` branches and a `spec` gate with no review lands on checks (`yes-no-review`). Reuses the stated route rather than adding a second (`reuse-first`).
2. **A `spec/<id>` branch with a `change.md` holding only an intent.** Rejected: a change exists to carry deltas and burdens; one with neither is a record to apply and archive for nothing, and it would hold a place in `clm proposals` (`yagni`).
3. **A `spec/*` branch with no `change.md`.** Rejected: `spec/<id>` names a change by its ID, and `clm proposals` and `propose-claims` read `spec/*` branches for one; a branch with none muddies that reading for no gain.
4. **A code change: a hint from `clm gate` or `clm land` naming the route.** Rejected: nothing refuses such a branch, so there is no refusal to carry a hint, and the guide ships in the binary for other repositories (`guide-in-the-binary`).

## Outcome

- `CLAUDE.md`'s small-change bullet names the doc-only case: the gate prints `spec`, it lands on checks, and an edit whose gate prints `spec review` or that needs `specs/` or `changes/` starts on a spec branch.
- `docs/guide/how-it-works.md` states the same for adopters, the limit included, since an adopter's `CLAUDE.md` doesn't carry this repository's bullet and an edit to `docs/allowed_signers` gates `spec review` (`method-in-the-plugin`, from the architecture audit).
- Decision `small-change-no-claim` amended.
- `backlog/no-stated-route-for-a-guide-only-edit-that-needs-no-change` deleted.

## Constraints

- No requirement changes, so nothing is built; the change has no delta.
- An edit to `ARCHITECTURE.md` or `CAPABILITIES.md` needs review (`yes-no-review`) and stays on a spec branch with a change.
- A decision that settles new design goes with its change on a spec branch, as `CLAUDE.md` already says; this route is only for recording a choice already made or rewording one.

## Edge cases

- `clm land` leaves a branch outside `impl/*`, `spec/*` and `propose/*` on `origin` (`land/landed-branch-cleanup`), so the session deletes the one it pushed, as for a small code change.
- `docs/guide/` counts as code for the land's check key (`guide-in-the-binary`), so such a land still runs the code checks.

## Open questions

None.
