---
title: "Approval still judges a host none clone against the local main"
kind: deferred
components: [land.approval, land]
---

`clm-approval`'s `Repo::new` takes `origin/main` as the base only when `host` isn't `none`, and the local `main` otherwise, and `clm-land`'s own `Repo` and its `(local)` land output rest on the same flag. `land.approval/approval-from-base` now reads the base as `origin/main` on every host, since a `host = "none"` clone keeps its own remote (`host/none-keeps-a-remote`). Approval's code was kept as it stands because `clm-land`'s offline tests (`crates/clm-land/tests/offline.rs`, `push_retry.rs`) still land on a local `main` until `land/offline-land` is built. Once that requirement is built and `clm-land` no longer uses the flag, remove the `remote` field and the local-main branch from `clm-approval` (`Repo::new`, `fetch`, the head choice), so the base is always `origin/main`.
