---
title: "RFC 3339 times are parsed by three copies of one function"
kind: deferred
components: [orchestrator, replica, web.worker.platform]
origin: 20261122-wave-4-loop-by-the-rule
---

`clm-orchestrator`'s `places::unix_time` (crates/clm-orchestrator/src/places.rs:488, private), `clm-replica`'s `compute::unix_time` (crates/clm-replica/src/compute.rs:360) and `clm-web-platform`'s `install::unix_time` (crates/clm-web-platform/src/install.rs:92, public, in a host crate `replica` may not depend on) each parse an RFC 3339 time, and they differ: `replica`'s read an offset as UTC until wave 4's build. Expected: one host-neutral provider (`ledger`, which writes the `at` they read) offers the parse as a call its Contract states, and the three use it.
