---
title: "The rule for which files the web replica fetches is held again by the Worker platform"
kind: deferred
components: [web.worker.platform]
origin: 20261122-wave-4-loop-by-the-rule
---

`replica` now offers its rule as `clm_replica::keeps_text(path)` and `clm_replica::wanted(path)` (`replica/files-it-fetches`). `web.worker.platform`'s `files::is_spec_path` and `is_size_path` (crates/clm-web-platform/src/files.rs:32, 268) hold a copy. If it drifts, what the platform lists or fetches disagrees with what the replica waits for. `web.worker.platform` adopts the call with a requirement that Needs `replica/files-it-fetches`, adding `replica` to its `depends_on` (no cycle: `replica` may not depend on it, `replica/host-neutral`). `web.worker.replica`'s own copy, `source::is_spec` and `source::wanted`, goes with `20261122-the-worker-source-reads`'s `web.worker.replica/fetches-what-the-replica-wants`.
