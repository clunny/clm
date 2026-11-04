---
title: "With a remote configured, main has one copy: clm land never lands locally"
date: 2026-10-26
status: accepted
---

Refines `land-locally`. A repository with `host = "none"` has no remote, so `clm land` lands on the local `main` and `clm reconnect` pushes it, with the ledgers, if a remote is ever added. A repository with any other host has exactly one `main`, the remote's: `clm land` fetches it, merges into it and pushes with a lease, and when `origin` can't be reached it refuses rather than landing locally.

Why: two people landing locally while offline make two `main`s. Reconciling them later means either re-landing one person's work (wasted, and their `landed` records name commits that never reach the shared `main`) or merging the two `main`s through a merge nobody checked, which breaks `land-locally`'s rule that every tree on `main` was checked by the lander. Refusing costs a person offline only the landing: they still build, commit and `clm sync` on their branch, and land when they're back.

This is what keeps a team in step, together with what already holds: every land, sync, claim and run fetches first; a node claim lapses after four hours without a push; spec work is snapshotted to `autosync/` each turn; `clm checkpoint` won't pass with unpushed work. No `mode` setting is needed: `host = "none"` is the only case with no shared `main`.
