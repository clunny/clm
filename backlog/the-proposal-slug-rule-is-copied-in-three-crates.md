---
title: "The proposal slug rule is copied in two crates"
kind: deferred
components: [plugin.hooks, replica]
origin: 20261122-wave-4-loop-by-the-rule
---

`clm defect`'s slug (`crates/clm-hooks/src/defect.rs:114`, private, falling back to `defect`) and `replica`'s proposal slug (`crates/clm-replica/src/write.rs:111`, stated by `replica/validate-writes`) are one rule kept twice; `clm_ledger::slug` is a different one. `web.worker`'s third copy (`crates/clm-web/src/proposals.rs`) goes with `20261124-wave-6-worker-by-the-rule`'s build of `web.worker`, which needs only whether the slug would be empty. No provider offers the rule as a call, and `clm-hooks` doesn't build for wasm, so `replica` can't call it. Expected: one host-neutral provider (`model` or `ledger`) offers the rule as a call its Contract states, and the two use it.
