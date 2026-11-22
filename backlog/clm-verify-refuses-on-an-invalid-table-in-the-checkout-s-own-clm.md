---
title: "clm verify refuses on an invalid table in the checkout's own clm.toml, which it should never read"
kind: bug
components: [verify, config]
---

`clm verify` takes `host` from the CLM root's settings with `clm_config::load(Some(root), env)`, which parses the working tree's `clm.toml` whole, `[verify]`, `[affected]` and `[env]` included (`crates/clm-verify/src/lib.rs`, `Ctx::open`). So a checkout whose own `[verify]` is invalid (no `check`, say) makes every `clm verify` refuse with exit 1 naming the checkout's file, although `verify/settings-from-main` says it SHALL never take `[verify]` or `[affected]` from the working tree's `clm.toml`. `config` offers no call that loads one key, or that loads the root's layers for `host` while ignoring the rest, so the fix needs a `config` requirement (a call for `host` alone, or a load that keeps going past a table it won't use) before `verify` can move onto it. Found by the spec-auditor in `verify`'s wave-3 build.
