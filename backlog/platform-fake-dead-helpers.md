---
title: "Remove the code nothing uses from clm-web-platform"
kind: deferred
components: [web.worker.platform]
origin: 20261111-slim-flagged-specs
---

`web.worker.platform`'s Design says the fake "calls nothing of `clm-web` and writes no checkout", but `crates/clm-web-platform/src/fake.rs` still holds `write_checkout`, `version_of` (which writes a temporary checkout and calls `clm_model::load`) and `citing`, which nothing calls. `20261109-slim-flagged-specs` said its build would remove them, but that change edited only the Design, so no build ran. The crate holds more code no requirement states and nothing calls (`no-dead-code`):
- `live`'s `tips`, `tip`, `claims`, `claim_refs`, `events`, `activity_of`, `Claim`, `Tip`, `Activity` and `REF_PAGES` (`src/live.rs`; `clm-web-replica`'s `source.rs` holds the live copies of tips and events), with the uncited test `live.rs::nested_branches_are_left_out`, the `clm_orchestrator::ClaimRef` import and the crate's `clm-orchestrator` dependency;
- `files::listed`, `Listed`, `sizes_of`, `Fetched.binary` and `BINARY_PROBE` (`src/files.rs`; the NUL probe's one provider is `replica`), with `listed`, `tree` and `texts` made private where `spec_files` still uses them;
- `install::rfc3339` (`src/install.rs`), and `access::ask` made `pub(crate)`;
- in the fake: `write_checkout`, `version_of` and `citing`, the `FakeRepo` field `ref_times` (read, never set; keep the constant `NOW` where it was read), `LiveCall.kv_writes_before`, and the `fake` feature's `tempfile` dependency (and `clm-model`, if nothing else in the fake needs it).

`FakeRepo.compare_status` and `FakeRepo.modes` stay, as `20261124-wave-6-worker-by-the-rule` decides. `web.worker.platform`'s build in that change removes the rest, confirms with a build of the crates that import `fake::*` (`clm-web`, `clm-web-review`, `clm-web-replica`) that nothing else uses them, and deletes this item with the trailer `Resolves: backlog/platform-fake-dead-helpers` (`deferred-work-resolves-in-code`). Removing `clm-orchestrator` lets `orchestrator` leave the component's `depends_on`, by `backlog/the-web-worker-crates-depends-on-keep-components-their-builds-stop-importing`.
