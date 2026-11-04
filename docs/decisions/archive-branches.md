---
title: "Complete changes are archived on a nested branch impl/archive/<id>, never read as a claim"
date: 2026-10-26
status: accepted
---

When `clm checkpoint` lists complete changes that no build archived, `implement` archives them all on one branch `impl/archive/<id>`, `<id>` the first change the checkpoint lists, cut from `origin/main`: it runs `clm sync` there (`archive-on-sync`), pushes, and lands it with `clm land`. Refines `archive-on-sync`, which left the branch's name to the skill.

**Why.** The skill used `impl/archive`. The orchestrator reads each `origin/impl/<x>` as a claim on node `<x>`, so `clm status` showed a claim row `archive` that is no node, and a node named `archive` would have read as claimed. A name with a `/` after `impl/` is never a node ID, and the queue, `claim_refs` and `clm status` already skip such refs, so the archive branch claims nothing. It stays under `impl/`, so `clm sync` archives on it and `clm land`, `clm checkpoint` and the gate treat it as the code branch it is.

**Races.** Two `implement` sessions that finish together see the same complete changes and pick the same `<id>`. The first push creates the branch; the second is refused because it exists, and that session deletes its local branch and leaves the archive to the first. A later checkpoint with a different first change gets a different name, so a landed archive branch is never reused.

**Alternatives.** Filtering claims to the graph's node IDs would keep `impl/archive`, but `clm status` and the queue would need the graph wherever they read claim refs, and the two racing sessions would still push one name with no per-change identity. A branch outside `impl/` (`archive/<id>`) would need `clm sync`, `clm land` and `clm checkpoint` to learn a new prefix. A per-session name (a timestamp) would let two sessions archive the same changes on two branches, and the second land would conflict.
