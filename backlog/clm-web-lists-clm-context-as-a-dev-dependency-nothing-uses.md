---
title: "clm-web lists clm-context as a dev-dependency nothing uses"
kind: deferred
components: [web.worker]
origin: 20261121-wave-3-verify-by-the-rule
---

`crates/clm-web/Cargo.toml` lists `clm-context` under `[dev-dependencies]`, and no file of `clm-web`, its tests included, uses it: the crate's code and tests name no `clm_context` item. `20261121-wave-3-verify-by-the-rule` removed `context/bundle-from-text` and `context/context-builds-for-wasm`, so nothing of `clm-context` is meant for the web app at all. `web.worker`'s next build removes the line (`no-dead-code`) and deletes this item with a `Resolves:` trailer (`deferred-work-resolves-in-code`). `cargo-machete` doesn't report it, for a reason not found; if the build finds it, say so in its pull request.
