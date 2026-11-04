---
title: "Slug IDs and content hashes for concurrency"
date: 2026-10-21
status: accepted
---

Requirement IDs are human slugs, not sequential numbers, so parallel authors don't collide. Staleness of deltas, plans and claims uses content hashes (invariant `hash-based-staleness`). This is one mechanism for both "concurrent spec edits" and "a queued plan became obsolete".
