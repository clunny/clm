---
title: "Remove the fake's helpers and fields nothing uses from clm-web-platform"
kind: deferred
components: [web.worker.platform]
origin: 20261111-slim-flagged-specs
---

`web.worker.platform`'s Design says the fake "calls nothing of `clm-web` and writes no checkout", but `crates/clm-web-platform/src/fake.rs` still holds `write_checkout`, `version_of` (which writes a temporary checkout and calls `clm_model::load`) and `citing`, which nothing calls, and the `FakeRepo` fields `compare_status` and `ref_times`, which are read but never set. `20261109-slim-flagged-specs` said its build would remove them, but that change edited only the Design, so no build ran. Remove them, keep the constants they stood in for (`"ahead"`, `NOW`) where the fields were read, and drop the `fake` feature's `tempfile` dependency (and `clm-model`, if nothing else in the fake needs it). Confirm with a build of the crates that import `fake::*` (`clm-web`, `clm-web-review`, `clm-web-replica`) that nothing else uses them.
