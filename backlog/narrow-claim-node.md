---
title: "Make orchestrator's claim_node private now that clm run claims through claim_local"
kind: deferred
components: [orchestrator, orchestrator.run]
origin: 20261105-run-cannot-resume-its
---

`orchestrator.run` now claims through `claim_local`, so `clm_orchestrator::claim_node` has no caller outside the `clm-orchestrator` library. The build of `orchestrator.run` kept it public, with an `--allow` entry for it in `clm.toml`, because `orchestrator`'s Contract lists `claim_node` among its library calls. A spec change drops it from that list; then make `claim_node` private in `crates/clm-orchestrator/src/repo.rs` and its re-export in `lib.rs`, and delete the `--allow` entry and its comment in `clm.toml`.
