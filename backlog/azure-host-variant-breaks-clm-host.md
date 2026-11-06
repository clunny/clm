---
title: "Adding the `azure` host value breaks `clm-host`'s exhaustive match on `Host`"
kind: question
components: [config, host]
blocks: [config/host-setting]
---

`config/host-setting` adds the value `azure` to `host`, so `clm_config::Host` gains a variant `Azure`. `clm-host`'s `adapter(host)` (`crates/clm-host/src/lib.rs`) matches `Host` exhaustively, so the new variant stops it compiling. An arm for an added variant that another component matches exhaustively is not a forced fix (`path-ownership`): which adapter `azure` selects is `host`'s decision, and `host/azure-pull-requests` is still unbuilt. So `config` leaves `host-setting` unbuilt and keeps `host` as it stands (`github`, `git`, `none`).

Every `host` requirement that waits on `config/host-setting` (`azure-pull-requests`, `azure-other-operations`, `host-for-remote`, `host-setup-steps`) therefore waits on this build, which can't land without the arm.

Needed from a spec branch: `docs/decisions/forced-fixes-cover-additions.md` and `path-ownership` rule out the placeholder arm `Host::Azure => Box::new(NoHost)` as a forced fix, since the arm is behaviour. Settling this means a spec change that lets the arm land with `config/host-setting` (for example an exception for a variant whose own requirement is already specced in the other component, naming `host/azure-pull-requests`), or a different order of the two builds that does not need the variant to exist first. A `host` requirement alone cannot add the arm before the variant exists. The same change should name the other places the variant reaches: `clm init --host` in `crates/clm/src/main.rs` matches its values as strings with `unreachable!` for any other, and `clm-init`'s host detection. Whatever spec branch settles it should list this item under `resolves`; `config/host-setting` then adds `Host::Azure`, its scenario, and re-cites the markers at `crates/clm/tests/cli.rs` and `crates/clm-config/tests/config.rs`.
