---
title: "Approvals, checks, lands and reviews are records in per-writer ledger branches"
date: 2026-10-26
status: accepted
---

The facts CLM's flow needs beyond the code itself, that a diff was approved, that a tree passed the checks, that a branch landed at a tier, that a SOFT land was reviewed, lived in GitHub as two labels and a bot comment. They now live in git as records: one JSON line each, in `records.jsonl` on a branch `clm/ledger/<person>/<clone>` that only that writer commits to. The state is the fold over the union of every ledger: a grow-only set of commutative facts, a CRDT in git. Two readers always compute the same approvals, checks, lands and reviews, offline included, and `web` reads the same files.

Why per-writer branches rather than one ledger or refs:
- **One shared file** (with `merge=union`) works, but every append contends on one tip and retries. Giving each writer its own branch removes the race entirely: nobody else pushes it, so an append is never rebased and never force-pushed. The writer unit is the clone, because git already serialises a clone's worktrees and sessions on its local ref; many sessions per person and many machines per person never meet.
- **Custom refs** (`refs/clm/*`) are atomic too, but invisible on every host's UI, blocked by some hosts' ref policies, absent from a default fetch, and gone without history when pruned. A branch is the one thing every host shows, syncs and permits.

What stays a branch with git's compare-and-swap is what genuinely needs mutual exclusion, which a CRDT can't give: claims (`impl/<node>`, `propose/<slug>`, where the branch is also the work) and `main`, which only `clm land` pushes.

The records replace `pinned-approval`'s bot comment (the `approved` record carries the fingerprint, so a rebase keeps it and a content change orphans it) and `soft-review-after-merge`'s label (a SOFT `landed` record with no `reviewed` record is what awaits review). Writers prune their own superseded `approved` and `checked` records; `landed` and `reviewed` are `main`'s audit trail, and `clm ledger verify` checks `main` against them.

A writer name is a claim, not a proof: a repository that lists its signers makes approvals and lands signed (`signed-ledger-records`); nothing proved who added a label either.

## The rule in full

`ARCHITECTURE.md` stated this as the entry `per-writer-ledgers` until 2026-11-13, when it moved down here; its rules serve `git-is-truth` (`human-layer-entries-stay-short`). The text still holds:

Approvals, checks and lands SHALL be records in per-writer ledger branches that only their writer appends to, so appends never race, rebase or force-push.

**Scenario: two lands at once**
- GIVEN two people landing at the same time
- WHEN each records its land
- THEN each appends to its own ledger branch and neither rewrites the other's
