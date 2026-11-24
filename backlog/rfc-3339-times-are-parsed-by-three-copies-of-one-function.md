---
title: "RFC 3339 times are parsed by two copies of ledger's function"
kind: deferred
components: [orchestrator, replica]
origin: 20261122-wave-4-loop-by-the-rule
---

`clm-orchestrator`'s `places::unix_time` (crates/clm-orchestrator/src/places.rs:488, private) and `clm-replica`'s `compute::unix_time` (crates/clm-replica/src/compute.rs:361) each parse an RFC 3339 time, and they can drift: `replica`'s read an offset as UTC until wave 4's build. `20261124-wave-6-worker-by-the-rule` gives the parse one provider, `ledger/record-times` (`clm_ledger::unix_time`, built with default features off), and moves `clm-web-platform`'s copy onto it. Expected: `orchestrator` and `replica` use `clm_ledger::unix_time` too, each with a requirement that Needs `ledger/record-times`, and delete their copies.
