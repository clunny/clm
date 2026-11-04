---
title: "clm sync keeps main's specs/ on any branch that adds a change"
date: 2026-10-27
status: accepted
---

`regenerate-applied-specs` made `clm sync` keep `main`'s side of `specs/` during the rebase only when the branch adds an `applied` change. A spec branch's history holds its `Apply` commits whatever the change's status at `HEAD`. An author who edits the deltas after the builder check sets the change back to `proposed` and restores `specs/` to apply it again, and syncs in between. That sync replayed the `Apply` commit over a `main` that had changed the same `SPEC.md` with a plain line merge, and stopped on a conflict in a generated file. The answer was always to take `main`'s side.

So the merge driver now covers `specs/` whenever the branch adds a change, whatever its status. Conflicts the driver can't take, such as modify/delete, are resolved the same way, also when the same stop has conflicts outside `specs/`. A conflict under `specs/` is never reported on such a branch. Only `applied` changes are then regenerated (`regenerate-applied-specs`). A branch whose added changes are all unapplied keeps `specs/` as the rebase made it, and `clm check` and the next `clm apply` judge it.

`specs/` on a spec branch is never hand-written (`regenerate-dont-merge`), so keeping `main`'s side loses no work there. A branch that adds no change, such as a finish branch setting `status: active` or an `impl/*` branch's `Activate` commit, still rebases `specs/` as text. Those edits are the branch's own, and nothing would regenerate them.

Rejected:
- **Keep `main`'s `specs/` on every branch.** That would drop a finish branch's status edit silently.
- **Reset `specs/` to `origin/main`'s after the rebase when no added change is `applied`.** This is tidier, but it would erase a branch's `specs/` edits that the rebase carried cleanly, for example on a branch `clm init` wrote. The reported failure needs only the conflicts resolved.
