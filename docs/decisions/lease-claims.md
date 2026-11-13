---
title: "Claims are pushed with --force-with-lease"
date: 2026-10-22
status: accepted
---

Refines `branch-claims`. Pushing a commit that already exists to a branch that already holds it succeeds, so "push a branch" alone isn't a lock. `clm claim` instead pushes a fresh empty commit with `--force-with-lease`, expecting no branch (a new claim) or the lapsed tip (a takeover). Git applies it as a compare-and-swap, so of two racing sessions exactly one wins. The empty commit's date also starts the heartbeat.

The expiry is a flag (`--expiry`, default 4 hours) for now. If teams need a shared setting, it moves into a committed config file.

## The rule in full

`ARCHITECTURE.md` stated this as the entry `lease-claims` until 2026-11-13, when it moved down here; its rules serve the capability `work-concurrently` (`human-layer-entries-stay-short`). The text still holds:

Claims SHALL be compare-and-swap pushes with an expiry and takeover, so that of two sessions racing for the same component or backlog item exactly one wins.

**Scenario: race**
- GIVEN two sessions claiming component `a` at once
- WHEN both push their claims
- THEN exactly one claim holds
