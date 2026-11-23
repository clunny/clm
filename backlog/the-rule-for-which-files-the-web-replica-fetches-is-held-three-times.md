---
title: "The rule for which files the web replica fetches is held three times"
kind: deferred
components: [web.worker.replica, web.worker.platform]
origin: 20261122-wave-4-loop-by-the-rule
---

`replica` now offers its rule as `clm_replica::keeps_text(path)` and `clm_replica::wanted(path)` (`replica/files-it-fetches`). `web.worker.replica`'s `source::is_spec` and `source::wanted` (crates/clm-web-replica/src/source.rs:43-55) and `web.worker.platform`'s `files::is_spec_path` and `is_size_path` (crates/clm-web-platform/src/files.rs:32, 268) each hold a copy. If a copy drifts, the source never fetches a file the replica waits for and `pending_fetch` never empties. Each adopts the call in wave 6 with a requirement that Needs `replica/files-it-fetches`; `web.worker.platform` adds `replica` to its `depends_on` (no cycle: `replica` may not depend on it, `replica/host-neutral`).
