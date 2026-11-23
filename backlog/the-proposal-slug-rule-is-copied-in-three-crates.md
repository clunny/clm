---
title: "The proposal slug rule is copied in three crates"
kind: deferred
components: [plugin.hooks, replica, web.worker]
origin: 20261122-wave-4-loop-by-the-rule
---

`clm defect`'s slug (`crates/clm-hooks/src/defect.rs:114`, private, falling back to `defect`), `replica`'s proposal slug (`crates/clm-replica/src/write.rs:111`, stated by `replica/validate-writes`) and `web.worker`'s (`crates/clm-web/src/proposals.rs:82`) are one rule kept three times; `clm_ledger::slug` is a different one. No provider offers it as a call, and `clm-hooks` doesn't build for wasm, so `replica` can't call it. Expected: one host-neutral provider (`model` or `ledger`) offers the rule as a call its Contract states, and the three use it.
