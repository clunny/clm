---
title: "The Worker's replica source reads times and picks files by its own copies of the replica's rules"
kind: bug
components: [web.worker.replica, web.worker.platform]
---

Found by the code review of `impl/replica` (wave 4).

1. `crates/clm-web-platform/src/install.rs`'s `unix_time` (about line 92), which `clm-web-replica`'s source uses for ref and event times, reads no offset, checks no digits and accepts trailing text: `2026-10-01T12:00:00+02:00` is taken as 12:00 UTC, two hours late, which moves claim lapse times (`recompute().next`) and the activity feed's order. `clm-replica`'s own parser now reads a whole RFC 3339 time (`replica/implementation-ledger`); the source should read times the same way.
2. `crates/clm-web-replica/src/source.rs` (`is_spec`, `wanted`, about line 53) keeps its own copy of which files to fetch, on `clm_web_platform::files::is_spec_path`, though `clm_replica::keeps_text` and `clm_replica::wanted` are now public as the rule a source fetches by (`replica/files-it-fetches`). If the two drift, the source never fetches a file `pending_fetch` lists and the replica stays `Preparing` for good. The source should call `clm_replica`'s rule and drop its copy.
