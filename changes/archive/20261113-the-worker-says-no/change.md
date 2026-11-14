---
status: done
resolves: []
---

## Intent

Set the backlog item `the-worker-says-no-review-for-an-exemption-the-gate-sends-to-review` from `kind: bug` to `kind: deferred`, and design nothing. No spec, code or skill text changes.

The item is not fixed on `main`: `needs_review_of` (`crates/clm-gate/src/policy.rs`) still has no exemption term, and the Worker still decides by it. The human settled its fix on 2026-11-13 (move the exemption comparison into the shared pure function) and, in the same answer, that its spec change "waits for the freeze in `backlog/a-passing-check-proves-less-than-the-spec-claims` to lift". That plan is at phase 3 of 8, its freeze is on, and it says an exception to the freeze is the human's to make, never a session's. So this draft does not design the change.

Decided work left for later is what `kind: deferred` is for. As a `bug` the item is drafted again by every triage, each draft a whole session that ends where this one did. As `deferred` it prints as a line in `clm triage` until the human judges it due, and the draft it then gets designs the settled fix. The item's text is unchanged, and it stays out of `resolves`, since nothing here fixes it.

Nothing unsafe lands meanwhile: `clm land` goes by the gate, which sends an exemption-only branch to review. What stays wrong until the freeze lifts is the web app's answer for such a branch, and that it can't offer the approval for it.
