---
title: "A proposing session claims its backlog items and checks in-flight spec work"
date: 2026-10-22
status: accepted
---

Extends `branch-claims` and `lease-claims` to design work. Parallel `propose` sessions picked from `clm backlog`, which lists an item until the spec PR that resolves it merges, so two sessions could design the same fix, and the human paid for both: in session time and in a second review.

A proposing session now claims each backlog item it works from with `clm claim backlog/<slug>`, which pushes `propose/<slug>` with the same compare-and-swap, expiry and takeover as a task claim. Of two racing sessions exactly one wins. The claim only has to last until the session's spec branch names the item: from then on that spec work holds it, read from `origin/spec/<id>` and from the `autosync/spec/<id>` snapshots the `Stop` hook pushes each turn. Spec work counts whatever its age, because a HARD PR can wait days for approval. A live change on `main` that `resolves` the item holds it too, until the task that fixes it deletes it. `clm proposals` lists all of this with each change's nodes, and `propose` shows the human any change that touches its nodes and asks what to do.

Overlap on nodes only warns. Locking nodes would let one session design on `orchestrator` at a time, and `clm sync` and hash bases already catch real conflicts when the second change lands.

Alternatives considered:
- The spec branch alone as the claim. Two sessions with different change IDs can both name the same item, and the loser shows only after both pushed: no atomic lock, which is the race to prevent.
- A claim field in the backlog item. Each claim would need a merged PR, and editing an existing item makes a code PR mixed (`gate/backlog-additions`).
- Showing taken items in `clm backlog`. `check` reads only the working tree and can't depend on `orchestrator`, and the web snapshot must match `clm backlog`, so `clm proposals` reports them instead.

Abandoned spec work holds its items until its branches are deleted. `clm proposals` names the change, so the human can see what to delete.
