---
title: "A needed contract change pauses only the affected tasks"
date: 2026-10-21
status: superseded
superseded_by: requirement-blocks
---

When a task finds that a contract must change, it stops, raises a new spec change, and is marked `blocked` with that change's ID. Tasks for nodes that depend on the blocked node also wait. Independent tasks continue. When the spec change merges, the affected bundle hashes change, so the blocked tasks show as stale and are re-planned (`hash-based-staleness`). No separate unblock step is needed.
