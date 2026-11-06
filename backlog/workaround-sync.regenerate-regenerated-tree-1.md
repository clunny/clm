---
title: "Tests fake clm sync because the node can't depend on sync"
kind: workaround
components: [sync.regenerate]
requirement: sync.regenerate/regenerated-tree@e590dd2e
---

The scenarios say 'clm sync runs', but clm-sync and clm-health aren't in this node's depends_on (check/undeclared-imports rejects them as dev-dependencies). The tests drive a small stand-in for sync's steps (fetch, rebase, regenerated, mirror, other_deltas, check) written in tests/regenerate.rs, so they prove the crate but not clm-sync's wiring, which sync's own tests cover.
