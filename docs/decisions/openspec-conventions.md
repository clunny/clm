---
title: "OpenSpec conventions, not the OpenSpec CLI"
date: 2026-10-21
status: accepted
---

We adopt requirement/scenario blocks and ADDED/MODIFIED/REMOVED deltas. We don't depend on the Node CLI, because we need node metadata (owners, deps, paths) and hash-based staleness it doesn't model, and our core is Rust anyway.
