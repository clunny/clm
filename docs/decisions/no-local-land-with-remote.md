---
title: "With a remote configured, main has one copy: clm land never lands locally"
date: 2026-10-26
status: accepted
---

Refines `land-locally`. A repository has exactly one `main`, the remote's: `clm land` fetches it, merges into it and pushes with a lease, and when `origin` can't be reached it refuses rather than landing locally. A repository with `host = "none"` is no exception: its clone keeps a remote of its own, a bare repository in its common git directory that every worktree shares, and `clm reconnect` pushes `main` and the ledgers to it when it is first made, or to a real remote that replaces it (`none-keeps-its-own-remote`).

Why: two people landing locally while offline make two `main`s. Reconciling them later means either re-landing one person's work (wasted, and their `landed` records name commits that never reach the shared `main`) or merging the two `main`s through a merge nobody checked, which breaks `land-locally`'s rule that every tree on `main` was checked by the lander. Refusing costs a person offline only the landing: they still build, commit and `clm sync` on their branch, and land when they're back.

This is what keeps a team in step, together with what already holds: every land, sync, claim and run fetches first; a node claim lapses after four hours without a push; spec work is snapshotted to `autosync/` each turn; `clm checkpoint` won't pass with unpushed work. No `mode` setting is needed: no host is left without a shared `main`.

## The rule in full

`ARCHITECTURE.md`'s entry `land-locally` states this in short (`human-layer-entries-stay-short`); the rule in full:

In a repository with a remote, `main` SHALL have one copy, the remote's: `clm land` refuses rather than land locally when the remote can't be reached.

**Scenario: remote unreachable**
- GIVEN a repository with a remote that can't be reached
- WHEN `clm land` runs
- THEN it exits non-zero and `main` is unchanged
