---
title: "Adding the `azure` host value breaks `clm-host`'s exhaustive match on `Host`"
kind: question
components: [config, host]
blocks: [config/host-setting]
---

`config/host-setting` adds the value `azure` to `host`, so `clm_config::Host` gains a variant `Azure`. `clm-host`'s `adapter(host)` (`crates/clm-host/src/lib.rs`) matches `Host` exhaustively, so the new variant stops it compiling. An arm for an added variant that another component matches exhaustively is not a forced fix (`path-ownership`): which adapter `azure` selects is `host`'s decision, and `host/azure-pull-requests` is still unbuilt. So `config` leaves `host-setting` unbuilt and keeps `host` as it stands (`github`, `git`, `none`).

Every `host` requirement that waits on `config/host-setting` (`azure-pull-requests`, `azure-other-operations`, `host-for-remote`, `host-setup-steps`) therefore waits on this build, which can't land without the arm.

Needed from a spec branch: either let `config`'s build add the placeholder arm `Host::Azure => Box::new(NoHost)` to `adapter` as a named forced fix, which `host/azure-pull-requests` then replaces, or give `host` a requirement that introduces the `Azure` arm and order it before `config/host-setting` with a `#### Needs` in a way that doesn't cycle. Once settled, delete this item; `config/host-setting` then adds `Host::Azure`, its scenario, and re-cites the markers at `crates/clm/tests/cli.rs` and `crates/clm-config/tests/config.rs`.
